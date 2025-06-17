use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct FileSaver {
    sender: Option<mpsc::Sender<(PathBuf, String)>>,
    handle: Option<thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl FileSaver {
    pub fn new() -> Self {
        if let Err(e) = fs::create_dir_all("save") {
            eprintln!("Failed to create save directory: {}", e);
        }

        let (sender, receiver) = mpsc::channel::<(PathBuf, String)>();

        let handle = thread::Builder::new()
            .name("file-saver".into())
            .spawn(move || {
                for (filename, content) in receiver {
                    let full_path = PathBuf::from("save").join(filename);
                    if let Err(e) = Self::write_file(&full_path, &content) {
                        eprintln!("Failed to write file {:?}: {}", full_path, e);
                    }
                }
                println!("File saver thread shutting down");
            })
            .expect("Failed to spawn file saver thread");

        FileSaver {
            sender: Some(sender),
            handle: Some(handle),
        }
    }

    fn write_file(filename: &PathBuf, content: &str) -> std::io::Result<()> {
        let mut writer = BufWriter::new(File::create(filename)?);
        writer.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn save(
        &self,
        filename: impl Into<PathBuf>,
        content: impl Into<String>,
    ) -> Result<(), String> {
        self.sender
            .as_ref()
            .ok_or_else(|| "FileSaver has been shutdown".to_string())?
            .send((filename.into(), content.into()))
            .map_err(|e| e.to_string())
    }

    pub fn shutdown(mut self) -> thread::Result<()> {
        self.sender = None;
        self.handle.take().unwrap().join()
    }
}

impl Drop for FileSaver {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.sender = None;
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
}

/// Asynchronous file loader that reads files in a background thread
#[allow(dead_code)]
pub struct FileLoader {
    sender: mpsc::Sender<(PathBuf, mpsc::Sender<String>)>,
    handle: thread::JoinHandle<()>,
}

#[allow(dead_code)]
impl FileLoader {
    /// Creates a new FileLoader with a dedicated background thread
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<(PathBuf, mpsc::Sender<String>)>();

        let handle = thread::Builder::new()
            .name("file-loader".into())
            .spawn(move || {
                for (filename, result_sender) in receiver {
                    match Self::read_file(&filename) {
                        Ok(content) => {
                            let _ = result_sender.send(content);
                        }
                        Err(e) => {
                            eprintln!("Failed to read file {:?}: {}", filename, e);
                            let _ = result_sender.send(String::new());
                        }
                    }
                }
                println!("File loader thread shutting down");
            })
            .expect("Failed to spawn file loader thread");

        FileLoader { sender, handle }
    }

    /// Internal helper method for file reading
    fn read_file(filename: &PathBuf) -> std::io::Result<String> {
        fs::read_to_string(filename)
    }

    /// Loads content from a file asynchronously
    pub fn load(&self, filename: impl Into<PathBuf>) -> mpsc::Receiver<String> {
        let (result_sender, result_receiver) = mpsc::channel();
        let _ = self.sender.send((filename.into(), result_sender));
        result_receiver
    }

    /// Shuts down the file loader
    pub fn shutdown(self) -> thread::Result<()> {
        drop(self.sender); // Close the channel
        self.handle.join()
    }
}

// Implement Clone for FileSaver to allow sharing between threads
impl Clone for FileSaver {
    fn clone(&self) -> Self {
        FileSaver {
            sender: self.sender.clone(),
            handle: None, // We don't clone the handle
        }
    }
}

// Example world list - in a real app you would scan a directory
pub const WORDS: &[&str] = &["World 1", "Adventure World", "Test World"];

#[allow(dead_code)]
fn main() {
    let saver = FileSaver::new();

    // Spawn a thread to create 100 files
    let saver_clone = saver.clone();
    thread::spawn(move || {
        for i in 1..=100 {
            let filename = format!("file_{}.txt", i);
            let content = format!("This is file number {}", i);
            saver_clone
                .save(filename, content)
                .expect("Failed to send save request");
            thread::sleep(Duration::from_millis(10)); // Small delay to see parallelism
        }
    });

    // Main thread prints messages while files are being created
    for i in 1..=5 {
        println!("Main thread working... while saving {}", i);
        thread::sleep(Duration::from_millis(100));
    }

    saver.shutdown().unwrap();

    let loader = FileLoader::new();

    // Load files in parallel
    let mut receivers = vec![];
    for i in 1..=100 {
        let rx = loader.load(format!("save/file_{}.txt", i));
        receivers.push((i, rx));
    }

    // Main thread prints messages while files are being created
    for i in 1..=5 {
        println!("Main thread working... while loading {}", i);
        thread::sleep(Duration::from_millis(100));
    }

    // Collect results
    for (i, rx) in receivers {
        match rx.recv() {
            #[allow(unused_variables)]
            Ok(content) => (), //println!("Loaded file {}: {}", i, content.trim()),
            Err(e) => eprintln!("Error loading file {}: {}", i, e),
        }
    }

    loader.shutdown().unwrap();

    println!("Main thread exiting");
}
