this folder contains the file system related stuffs
like assets sounds etc just the neat things, even the fonts (if you go with the default)


/// Handles file system operations, including loading resources, reading from disk, and serialization.
pub mod fs {
    /// Compiled resources embedded in the binary.
    pub mod rs;
    /// File system operations (reading/writing to disk).
    pub mod fs;
    /// Custom JSON parser (alternative to `serde_json`).
    pub mod json;
    /// Binary serialization utilities.
    pub mod binary;
}

