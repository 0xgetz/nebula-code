use std::fs;
use std::path::Path;
use std::io::{self, Read, Write};

/// Read the contents of a file
pub fn read_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Write contents to a file, creating it if it doesn't exist
pub fn write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, contents)
}

/// Ensure a directory exists, creating it and any missing parent directories
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::create_dir_all(path)
}

/// Check if a file exists
pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Get the size of a file in bytes
pub fn file_size<P: AsRef<Path>>(path: P) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}
