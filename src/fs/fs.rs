
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

/// Optimized asynchronous file saver with better error handling and performance
pub struct FileSaver {
	sender: Option<mpsc::Sender<FileOperation>>,
	handle: Option<thread::JoinHandle<()>>,
}

/// Optimized asynchronous file loader with shared thread pool concept
pub struct FileLoader {
	sender: mpsc::Sender<LoadOperation>,
	handle: thread::JoinHandle<()>,
}

#[derive(Debug)]
#[allow(dead_code)]
enum FileOperation {
	Save { path: PathBuf, content: String },
	SaveIfEmpty { path: PathBuf, content: String },
}

#[derive(Debug)]
#[allow(dead_code)]
struct LoadOperation {
	path: PathBuf,
	response: mpsc::Sender<std::io::Result<String>>,
}

#[allow(dead_code)]
impl FileSaver {
	pub fn new() -> std::io::Result<Self> {
		std::fs::create_dir_all("save")?;
		
		let (sender, receiver) = mpsc::channel();
		let handle = thread::Builder::new()
			.name("file-saver".into())
			.spawn(move || {
				// Use BufWriter for better performance with multiple small writes
				for operation in receiver {
					match operation {
						FileOperation::Save { path, content } => {
							if let Err(e) = Self::write_file_atomic(&path, &content) {
								eprintln!("Failed to write file {:?}: {}", path, e);
							}
						}
						FileOperation::SaveIfEmpty { path, content } => {
							if !Self::file_has_content(&path) {
								if let Err(e) = Self::write_file_atomic(&path, &content) {
									eprintln!("Failed to write file {:?}: {}", path, e);
								}
							}
						}
					}
				}
			})?;

		Ok(FileSaver {
			sender: Some(sender),
			handle: Some(handle),
		})
	}

	/// Atomic file write using temporary file + rename for safety
	fn write_file_atomic(path: &Path, content: &str) -> std::io::Result<()> {
		let full_path = PathBuf::from("save").join(path);
		let temp_path = full_path.with_extension("tmp");
		
		// Ensure parent directory exists
		if let Some(parent) = full_path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		
		// Write to temporary file first
		{
			let mut writer = std::io::BufWriter::new(std::fs::File::create(&temp_path)?);
			writer.write_all(content.as_bytes())?;
			writer.flush()?;
		} // BufWriter is dropped here, ensuring all data is written
		
		// Atomically move temporary file to final location
		std::fs::rename(temp_path, full_path)?;
		Ok(())
	}

	fn file_has_content(path: &Path) -> bool {
		let full_path = PathBuf::from("save").join(path);
		std::fs::read_to_string(&full_path)
			.map(|content| !content.trim().is_empty())
			.unwrap_or(false)
	}

	pub fn save(&self, filename: impl AsRef<Path>, content: impl Into<String>) -> Result<(), &'static str> {
		let operation = FileOperation::Save {
			path: filename.as_ref().to_path_buf(),
			content: content.into(),
		};
		
		self.sender
			.as_ref()
			.ok_or("FileSaver has been shutdown")?
			.send(operation)
			.map_err(|_| "Failed to send save operation")
	}

	pub fn save_if_empty(&self, filename: impl AsRef<Path>, content: impl Into<String>) -> Result<(), &'static str> {
		let operation = FileOperation::SaveIfEmpty {
			path: filename.as_ref().to_path_buf(),
			content: content.into(),
		};
		
		self.sender
			.as_ref()
			.ok_or("FileSaver has been shutdown")?
			.send(operation)
			.map_err(|_| "Failed to send save operation")
	}

	/// Synchronously read file or return default
	pub fn read_or_default(&self, filename: impl AsRef<Path>, default: impl Into<String>) -> String {
		let full_path = PathBuf::from("save").join(filename.as_ref());
		match std::fs::read_to_string(full_path) {
			Ok(content) if !content.trim().is_empty() => content,
			_ => default.into(),
		}
	}

	pub fn shutdown(mut self) -> thread::Result<()> {
		drop(self.sender.take()); // Close channel
		self.handle.take().unwrap().join()
	}
}

#[allow(dead_code)]
impl FileLoader {
	pub fn new() -> std::io::Result<Self> {
		let (sender, receiver) = mpsc::channel();
		let handle = thread::Builder::new()
			.name("file-loader".into())
			.spawn(move || {
				for LoadOperation { path, response } in receiver {
					let full_path = PathBuf::from("save").join(&path);
					let result = std::fs::read_to_string(&full_path);
					let _ = response.send(result); // Ignore if receiver dropped
				}
			})?;

		Ok(FileLoader { sender, handle })
	}

	pub fn load(&self, filename: impl AsRef<Path>) -> mpsc::Receiver<std::io::Result<String>> {
		let (response_tx, response_rx) = mpsc::channel();
		let operation = LoadOperation {
			path: filename.as_ref().to_path_buf(),
			response: response_tx,
		};
		
		// If send fails, the thread is likely shut down
		if self.sender.send(operation).is_err() {
			let (tx, rx) = mpsc::channel();
			let _ = tx.send(Err(std::io::Error::new(
				std::io::ErrorKind::BrokenPipe,
				"FileLoader has been shut down"
			)));
			return rx;
		}
		
		response_rx
	}

	/// Load with default value if file doesn't exist or is empty
	pub fn load_or_default(&self, filename: impl AsRef<Path>, default: String) -> String {
		match self.load(filename).recv() {
			Ok(Ok(content)) if !content.trim().is_empty() => content,
			_ => default,
		}
	}

	/// Load multiple files concurrently and return when all complete
	pub fn load_batch(&self, filenames: Vec<impl AsRef<Path>>) -> Vec<std::io::Result<String>> {
		let receivers: Vec<_> = filenames.into_iter()
			.map(|filename| self.load(filename))
			.collect();
		
		receivers.into_iter()
			.map(|rx| rx.recv().unwrap_or_else(|_| Err(std::io::Error::new(
				std::io::ErrorKind::BrokenPipe,
				"Channel closed"
			))))
			.collect()
	}

	pub fn shutdown(self) -> thread::Result<()> {
		drop(self.sender); // Close channel
		self.handle.join()
	}
}

// Safe clone implementation for FileSaver
impl Clone for FileSaver {
	fn clone(&self) -> Self {
		FileSaver {
			sender: self.sender.clone(),
			handle: None, // Don't clone the handle
		}
	}
}

impl Drop for FileSaver {
	fn drop(&mut self) {
		if let Some(handle) = self.handle.take() {
			drop(self.sender.take()); // Close channel
			let _ = handle.join(); // Wait for thread to finish
		}
	}
}
