use crate::ui::manager::{UIState, UIStateID, UIManager};
use crate::ext::ptr;
use std::{
	cell::RefCell,
	collections::HashMap,
	fmt,
	future::Future,
	pin::Pin,
	sync::{
		atomic::{AtomicU8, Ordering},
		Arc, Mutex, RwLock,
	},
	task::{Context, Poll, Waker},
	time::Instant,
};

// ============================================================================
// DialogManager (Without futures-lite and arc-swap)
// ============================================================================

/// Dialog manager for handling multiple concurrent dialogs
#[derive(Clone)]
pub struct DialogManager {
	inner: Arc<DialogManagerInner>,
}

struct DialogManagerInner {
	pending: RwLock<HashMap<u8, PendingDialog>>,
	counter: AtomicU8,
}

type DialogCallback = Arc<RefCell<dyn FnMut(bool) + 'static>>;

#[derive(Clone)]
struct PendingDialog {
	response_holder: Arc<Mutex<Option<bool>>>,
	prompt: String,
	created_at: Instant,
	callback: Option<DialogCallback>,
	waker: Arc<Mutex<Option<Waker>>>,
}

impl DialogManager {
	/// Creates a new dialog manager
	pub fn new() -> Self {
		Self {
			inner: Arc::new(DialogManagerInner {
				pending: RwLock::new(HashMap::new()),
				counter: AtomicU8::new(0),
			}),
		}
	}

	/// Shows a dialog and awaits user response
	pub async fn ask(&self, prompt: impl Into<String>) -> Result<bool, DialogError> {
		let prompt = prompt.into();
		let id: u8 = self.inner.counter.fetch_add(1, Ordering::Relaxed);
		let response_holder = Arc::new(Mutex::new(None));
		let waker = Arc::new(Mutex::new(None));

		// Add to pending dialogs
		if let Ok(mut pending) = self.inner.pending.write() {
			pending.insert(id, PendingDialog {
				response_holder: response_holder.clone(),
				prompt: prompt.clone(),
				created_at: Instant::now(),
				callback: None,
				waker: waker.clone(),
			});
		}

		// Show dialog in UI
		ptr::get_state().ui_manager.confirm(id, &prompt);

		// Create and await the future
		DialogFuture {
			response_holder,
			waker,
			dialog_manager: self.clone(),
			id,
		}.await
	}

	/// Shows a dialog with a callback (non-blocking)
	pub fn ask_with_callback<F: FnMut(bool) + 'static>(&self, prompt: impl Into<String>, callback: F) -> u8 {
		let prompt = prompt.into();
		let id: u8 = self.inner.counter.fetch_add(1, Ordering::Relaxed);
		let response_holder = Arc::new(Mutex::new(None));
		let waker = Arc::new(Mutex::new(None));
		
		// Wrap the callback in Arc<RefCell>
		let callback = Arc::new(RefCell::new(callback));

		// Add to pending dialogs with callback
		if let Ok(mut pending) = self.inner.pending.write() {
			pending.insert(id, PendingDialog {
				response_holder: response_holder.clone(),
				prompt: prompt.clone(),
				created_at: Instant::now(),
				callback: Some(callback.clone()),
				waker,
			});
		}

		// Show dialog in UI
		ptr::get_state().ui_manager.confirm(id, &prompt);

		id
	}

	/// Respond to a specific dialog by ID
	pub fn respond(&self, id: u8, response: bool) -> bool {
		if let Ok(pending_lock) = self.inner.pending.read() {
			if let Some(pending) = pending_lock.get(&id) {
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
				
				// Wake up any waiting futures
				if let Ok(mut waker_guard) = pending.waker.lock() {
					if let Some(waker) = waker_guard.take() {
						waker.wake();
					}
				}
				
				// Drop the read lock before removing
				drop(pending_lock);
				
				// Remove from pending
				self.remove_pending(id);
				return true;
			}
		}
		false
	}

	/// Get a pending dialog by ID
	pub fn get_pending_dialog(&self, id: u8) -> Option<String> {
		if let Ok(pending) = self.inner.pending.read() {
			pending.get(&id).map(|dialog| dialog.prompt.clone())
		} else {
			None
		}
	}

	/// Get all pending dialog IDs and their prompts
	pub fn get_pending_dialogs(&self) -> Vec<(u8, String, Instant)> {
		if let Ok(pending) = self.inner.pending.read() {
			pending
				.iter()
				.map(|(id, dialog)| (*id, dialog.prompt.clone(), dialog.created_at))
				.collect()
		} else {
			Vec::new()
		}
	}

	/// Cancel a specific dialog
	pub fn cancel_dialog(&self, id: u8) -> bool {
		if let Ok(pending_lock) = self.inner.pending.read() {
			if let Some(pending) = pending_lock.get(&id) {
				// Execute callback with false if present
				if let Some(ref callback) = pending.callback {
					if let Ok(mut callback_mut) = callback.try_borrow_mut() {
						callback_mut(false);
					}
				}
				
				if let Ok(mut guard) = pending.response_holder.lock() {
					*guard = Some(false);
				}
				
				// Wake up any waiting futures
				if let Ok(mut waker_guard) = pending.waker.lock() {
					if let Some(waker) = waker_guard.take() {
						waker.wake();
					}
				}
				
				// Drop the read lock before removing
				drop(pending_lock);
				
				self.remove_pending(id);
				return true;
			}
		}
		false
	}

	/// Cancel all pending dialogs
	pub fn cancel_all(&self) {
		if let Ok(mut pending) = self.inner.pending.write() {
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
				
				// Wake up any waiting futures
				if let Ok(mut waker_guard) = dialog.waker.lock() {
					if let Some(waker) = waker_guard.take() {
						waker.wake();
					}
				}
			}
			pending.clear();
		}
	}

	/// Get count of pending dialogs
	pub fn pending_count(&self) -> usize {
		if let Ok(pending) = self.inner.pending.read() {
			pending.len()
		} else {
			0
		}
	}

	fn remove_pending(&self, id: u8) {
		if let Ok(mut pending) = self.inner.pending.write() {
			pending.remove(&id);
		}
	}
}

impl Default for DialogManager {
	fn default() -> Self {
		Self::new()
	}
}

// ============================================================================
// Custom Future Implementation (replaces futures-lite::future::yield_now)
// ============================================================================

struct DialogFuture {
	response_holder: Arc<Mutex<Option<bool>>>,
	waker: Arc<Mutex<Option<Waker>>>,
	dialog_manager: DialogManager,
	id: u8,
}

impl Future for DialogFuture {
	type Output = Result<bool, DialogError>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		// Check if we have a response
		if let Ok(guard) = self.response_holder.try_lock() {
			if let Some(response) = *guard {
				// Clean up
				self.dialog_manager.remove_pending(self.id);
				return Poll::Ready(Ok(response));
			}
		}

		// Store the waker for later use
		if let Ok(mut waker_guard) = self.waker.lock() {
			*waker_guard = Some(cx.waker().clone());
		}

		Poll::Pending
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