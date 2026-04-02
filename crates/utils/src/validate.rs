use std::path::Path;

/// Validate a name (alphanumeric, hyphens, underscores)
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    
    if name.len() > 64 {
        return Err("Name must be 64 characters or less".to_string());
    }
    
    let valid_chars = name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if !valid_chars {
        return Err("Name can only contain alphanumeric characters, hyphens, and underscores".to_string());
    }
    
    if name.starts_with('-') || name.starts_with('_') {
        return Err("Name cannot start with a hyphen or underscore".to_string());
    }
    
    Ok(())
}

/// Validate a semantic version string
pub fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }
    
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return Err("Version must be in semver format (e.g., 1.0.0)".to_string());
    }
    
    for part in parts {
        if part.is_empty() {
            return Err("Version parts cannot be empty".to_string());
        }
        if part.parse::<u32>().is_err() {
            return Err("Version parts must be numeric".to_string());
        }
    }
    
    Ok(())
}

/// Validate a file system path
pub fn validate_path<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();
    
    if path.as_os_str().is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    
    // Check for invalid characters (basic check)
    let path_str = path.to_string_lossy();
    if path_str.contains('\0') {
        return Err("Path contains null character".to_string());
    }
    
    Ok(())
}
