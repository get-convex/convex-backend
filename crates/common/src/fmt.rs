use std::time::Duration;

/// Units ordered from largest to smallest, mixing binary and decimal.
/// We try each and pick the one that divides most cleanly.
const BYTE_UNITS: &[(u64, &str)] = &[
    (1 << 30, "GiB"),
    (1_000_000_000, "GB"),
    (1 << 20, "MiB"),
    (1_000_000, "MB"),
    (1 << 10, "KiB"),
    (1_000, "KB"),
];

/// Format a byte count into a human-friendly string.
///
/// Picks the unit (binary or decimal) that divides most cleanly.
/// Shows one decimal place only when it divides exactly (e.g. "4.1 MiB").
/// Falls back to raw bytes when no unit divides cleanly.
///
/// ```
/// use common::fmt::format_bytes;
/// assert_eq!(format_bytes(8_388_608), "8 MiB");
/// assert_eq!(format_bytes(1534), "1534 bytes");
/// assert_eq!(format_bytes(1024), "1 KiB");
/// assert_eq!(format_bytes(1000), "1 KB");
/// assert_eq!(format_bytes(0), "0 bytes");
/// assert_eq!(format_bytes(4_718_592), "4.5 MiB");
/// ```
pub fn format_bytes(n: u64) -> String {
    if n == 0 {
        return "0 bytes".to_string();
    }
    for &(unit_size, unit_name) in BYTE_UNITS {
        if n < unit_size {
            continue;
        }
        if n % unit_size == 0 {
            return format!("{} {unit_name}", n / unit_size);
        }
        if (n * 10) % unit_size == 0 {
            let whole = n / unit_size;
            let frac = (n * 10 / unit_size) % 10;
            return format!("{whole}.{frac} {unit_name}");
        }
    }
    format!("{n} bytes")
}

/// Format a duration into a human-friendly string like "1 second" or "500ms".
///
/// ```
/// use std::time::Duration;
/// use common::fmt::format_duration;
/// assert_eq!(format_duration(Duration::from_millis(1000)), "1 second");
/// assert_eq!(format_duration(Duration::from_millis(2000)), "2 seconds");
/// assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
/// assert_eq!(format_duration(Duration::from_millis(1500)), "1.5 seconds");
/// assert_eq!(format_duration(Duration::from_secs(60)), "60 seconds");
/// ```
pub fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms == 0 {
        return "0ms".to_string();
    }
    if ms >= 1000 {
        if ms % 1000 == 0 {
            let secs = ms / 1000;
            if secs == 1 {
                return "1 second".to_string();
            }
            return format!("{secs} seconds");
        }
        if (ms * 10) % 1000 == 0 {
            let whole = ms / 1000;
            let frac = (ms * 10 / 1000) % 10;
            return format!("{whole}.{frac} seconds");
        }
    }
    format!("{ms}ms")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        // Exact binary units
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(1), "1 bytes");
        assert_eq!(format_bytes(512), "512 bytes");
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(2048), "2 KiB");
        assert_eq!(format_bytes(1 << 20), "1 MiB");
        assert_eq!(format_bytes(8 << 20), "8 MiB");
        assert_eq!(format_bytes(1 << 30), "1 GiB");

        // Exact decimal units
        assert_eq!(format_bytes(1000), "1 KB");
        assert_eq!(format_bytes(1_000_000), "1 MB");
        assert_eq!(format_bytes(1_000_000_000), "1 GB");

        // One decimal, binary
        assert_eq!(format_bytes(1024 + 512), "1.5 KiB");
        // 4.2 MiB = 4 * 1048576 + 0.2 * 1048576 = 4194304 + 209715.2 -- not exact
        // 4.5 MiB = 4718592
        assert_eq!(format_bytes(4_718_592), "4.5 MiB");

        // Falls back to bytes when nothing divides cleanly
        assert_eq!(format_bytes(1534), "1534 bytes");
        assert_eq!(format_bytes(999), "999 bytes");

        // Prefers larger clean unit
        assert_eq!(format_bytes(1 << 20), "1 MiB"); // not "1024 KiB"

        // 512 KiB
        assert_eq!(format_bytes(512 * 1024), "512 KiB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
        assert_eq!(format_duration(Duration::from_millis(1)), "1ms");
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_millis(999)), "999ms");
        assert_eq!(format_duration(Duration::from_millis(1000)), "1 second");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.5 seconds");
        assert_eq!(format_duration(Duration::from_millis(2000)), "2 seconds");
        assert_eq!(format_duration(Duration::from_millis(5000)), "5 seconds");
        assert_eq!(format_duration(Duration::from_millis(1234)), "1234ms");
    }
}
