/// Format a size in bytes to a human-readable string
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    match bytes {
        b if b < KB => format!("{} B", b),
        b if b < MB => format!("{:.1} KB", b as f64 / KB as f64),
        b if b < GB => format!("{:.1} MB", b as f64 / MB as f64),
        b => format!("{:.2} GB", b as f64 / GB as f64),
    }
}

/// Format a duration in milliseconds to a human-readable string
pub fn format_duration(ms: u64) -> String {
    const SEC: u64 = 1000;
    const MIN: u64 = SEC * 60;
    const HR: u64 = MIN * 60;
    
    if ms < SEC {
        format!("{}ms", ms)
    } else if ms < MIN {
        format!("{:.1}s", ms as f64 / SEC as f64)
    } else if ms < HR {
        let minutes = ms / MIN;
        let seconds = (ms % MIN) / SEC;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = ms / HR;
        let minutes = (ms % HR) / MIN;
        format!("{}h {}m", hours, minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65000), "1m 5s");
    }
}
