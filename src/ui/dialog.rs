
use crate::ui::manager::{UIState, UIStateID, UIManager};
use crate::ext::ptr;
use arc_swap::ArcSwap;
use futures_lite::future;
use std::{
	cell::RefCell,
	collections::HashMap,
	fmt, sync::{
	atomic::{AtomicU8, Ordering},
	Arc, Mutex, }, time::Instant,
};

// ============================================================================
// DialogManager (With IDs - More complex but easier UI access)
// ============================================================================

/// Dialog manager for handling multiple concurrent dialogs
#[derive(Clone)]
pub struct DialogManager {
	inner: Arc<DialogManagerInner>,
}

struct DialogManagerInner {
	pending: ArcSwap<HashMap<u8, PendingDialog>>,
	counter: AtomicU8,
}

type DialogCallback = Arc<RefCell<dyn FnMut(bool) + 'static>>;

#[derive(Clone)]
struct PendingDialog {
	response_holder: Arc<Mutex<Option<bool>>>,
	prompt: String,
	created_at: Instant,
	callback: Option<DialogCallback>,
}

impl DialogManager {
	/// Creates a new dialog manager
	pub fn new() -> Self {
		Self {
			inner: Arc::new(DialogManagerInner {
				pending: ArcSwap::new(Arc::new(HashMap::new())),
				counter: AtomicU8::new(0),
			}),
		}
	}

	/// Shows a dialog and awaits user response
	pub async fn ask(&self, prompt: impl Into<String>) -> Result<bool, DialogError> {
		let prompt = prompt.into();
		let id: u8 = self.inner.counter.fetch_add(1, Ordering::Relaxed);
		let response_holder = Arc::new(Mutex::new(None));

		// Add to pending dialogs
		self.inner.pending.rcu(|pending| {
			let mut new = HashMap::clone(pending);
			new.insert(id, PendingDialog {
				response_holder: response_holder.clone(),
				prompt: prompt.clone(),
				created_at: Instant::now(),
				callback: None,
			});
			Arc::new(new)
		});

		// Show dialog in UI
		ptr::get_state().ui_manager.confirm(id, &prompt);

		// Wait for response
		loop {
			if let Ok(guard) = response_holder.try_lock() {
				if let Some(response) = *guard {
					self.remove_pending(id);
					return Ok(response);
				}
			}
			future::yield_now().await;
		}
	}

	/// Shows a dialog with a callback (non-blocking)
	pub fn ask_with_callback<F>(&self, prompt: impl Into<String>, callback: F) -> u8
	where
		F: FnMut(bool) + 'static,
	{
		let prompt = prompt.into();
		let id: u8 = self.inner.counter.fetch_add(1, Ordering::Relaxed);
		let response_holder = Arc::new(Mutex::new(None));
		
		// Wrap the callback in Arc<RefCell> before moving it into the closure
		let callback = Arc::new(RefCell::new(callback));

		// Add to pending dialogs with callback
		self.inner.pending.rcu(|pending| {
			let mut new = HashMap::clone(pending);
			new.insert(id, PendingDialog {
				response_holder: response_holder.clone(),
				prompt: prompt.clone(),
				created_at: Instant::now(),
				callback: Some(callback.clone()), // Now we're cloning the Arc
			});
			Arc::new(new)
		});

		// Show dialog in UI
		ptr::get_state().ui_manager.confirm(id, &prompt);

		id
	}

	/// Respond to a specific dialog by ID
	pub fn respond(&self, id: u8, response: bool) -> bool {
		if let Some(pending) = self.inner.pending.load().get(&id) {
			// Execute callback if present
			if let Some(ref callback) = pending.callback {
				if let Ok(mut callback_mut) = callback.try_borrow_mut() {
					callback_mut(response);
				}
			}
			
			// Set response for async waiters
			if let Ok(mut guard) = pending.response_holder.lock() {
				*guard = Some(response);
			}
			
			// Remove from pending
			self.remove_pending(id);
			return true;
		}
		false
	}

	/// Get a pending dialog by ID
	pub fn get_pending_dialog(&self, id: u8) -> Option<String> {
		if let Some(pending) = self.inner.pending.load().get(&id) {
			Some(pending.prompt.clone())
		} else {
			None
		}
	}

	/// Get all pending dialog IDs and their prompts
	pub fn get_pending_dialogs(&self) -> Vec<(u8, String, Instant)> {
		self.inner
			.pending
			.load()
			.iter()
			.map(|(id, dialog)| (*id, dialog.prompt.clone(), dialog.created_at))
			.collect()
	}

	/// Cancel a specific dialog
	pub fn cancel_dialog(&self, id: u8) -> bool {
		if let Some(pending) = self.inner.pending.load().get(&id) {
			// Execute callback with false if present
			if let Some(ref callback) = pending.callback {
				if let Ok(mut callback_mut) = callback.try_borrow_mut() {
					callback_mut(false);
				}
			}
			
			if let Ok(mut guard) = pending.response_holder.lock() {
				*guard = Some(false);
			}
			self.remove_pending(id);
			return true;
		}
		false
	}

	/// Cancel all pending dialogs
	pub fn cancel_all(&self) {
		let pending = self.inner.pending.load();
		for (_id, dialog) in pending.iter() {
			// Execute callbacks with false
			if let Some(ref callback) = dialog.callback {
				if let Ok(mut callback_mut) = callback.try_borrow_mut() {
					callback_mut(false);
				}
			}
			
			if let Ok(mut guard) = dialog.response_holder.lock() {
				*guard = Some(false);
			}
		}
		self.inner.pending.store(Arc::new(HashMap::new()));
	}

	/// Get count of pending dialogs
	pub fn pending_count(&self) -> usize {
		self.inner.pending.load().len()
	}

	fn remove_pending(&self, id: u8) {
		self.inner.pending.rcu(|pending| {
			let mut new = HashMap::clone(pending);
			new.remove(&id);
			Arc::new(new)
		});
	}
}

impl Default for DialogManager {
	fn default() -> Self {
		Self::new()
	}
}

// ============================================================================
// Common Error Types
// ============================================================================

/// Dialog interaction results
#[derive(Debug, Clone, PartialEq)]
pub enum DialogError {
	/// User explicitly closed the dialog without responding
	Dismissed {
		at: Instant,
		prompt: String,
	},
	/// Dialog system was shut down
	SystemShutdown,
	/// Custom application-specific errors
	Custom {
		kind: &'static str,
		details: String,
	},
}

impl DialogError {
	/// Create a dismissed error with context
	pub fn dismissed(prompt: impl Into<String>) -> Self {
		Self::Dismissed {
			at: Instant::now(),
			prompt: prompt.into(),
		}
	}

	/// Create a custom error
	pub fn custom(kind: &'static str, details: impl Into<String>) -> Self {
		Self::Custom {
			kind,
			details: details.into(),
		}
	}
}

impl fmt::Display for DialogError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Dismissed { prompt, .. } => write!(f, "Dialog dismissed: \"{prompt}\""),
			Self::SystemShutdown => write!(f, "Dialog system unavailable"),
			Self::Custom { kind, details } => write!(f, "[{kind}] {details}"),
		}
	}
}

impl std::error::Error for DialogError {}

impl UIManager {
	pub fn confirm(&mut self, id: u8, _prompt: impl Into<String>) {
		let ui_manager = &mut ptr::get_state().ui_manager;
		ui_manager.state = UIState::Confirm(UIStateID::from(&ui_manager.state), id.clone());
		ui_manager.setup_ui();
	}
}
