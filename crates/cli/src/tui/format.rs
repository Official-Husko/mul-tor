//! Human-readable numbers for the screen.

use mul_tor_core::format_bytes;

/// `3.2 MB/s`
pub fn speed(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second.max(0.0) as u64))
}

/// `0:07`, `12:34`, `1:02:03`; `--:--` when unknown.
pub fn duration(seconds: Option<u64>) -> String {
    let Some(total) = seconds else {
        return "--:--".to_string();
    };
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Whole percent of `part` in `total`, never above 100; 0 when `total` is 0.
pub fn percent(part: u64, total: u64) -> u16 {
    if total == 0 {
        0
    } else {
        ((part as u128 * 100) / total as u128).min(100) as u16
    }
}

/// `file` or `files`, after a count: `1 file`, `3 files`.
pub fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

/// Shorten `text` to `max` columns with an ellipsis in the middle, so both
/// the start of a name and its extension stay visible.
pub fn middle_ellipsis(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    if max <= 1 {
        return "…".repeat(max);
    }
    let keep = max - 1;
    let tail = keep / 2;
    let head = keep - tail;
    let start: String = chars[..head].iter().collect();
    let end: String = chars[chars.len() - tail..].iter().collect();
    format!("{start}…{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_use_the_shortest_clock_form() {
        assert_eq!(duration(Some(7)), "0:07");
        assert_eq!(duration(Some(754)), "12:34");
        assert_eq!(duration(Some(3723)), "1:02:03");
        assert_eq!(duration(None), "--:--");
    }

    #[test]
    fn percentages_never_exceed_a_hundred_or_divide_by_zero() {
        assert_eq!(percent(1, 4), 25);
        assert_eq!(percent(5, 4), 100);
        assert_eq!(percent(1, 0), 0);
        assert_eq!(percent(u64::MAX, u64::MAX), 100);
    }

    #[test]
    fn counts_agree_with_their_noun() {
        assert_eq!(count(1, "file", "files"), "1 file");
        assert_eq!(count(0, "file", "files"), "0 files");
    }

    #[test]
    fn long_names_keep_their_start_and_end() {
        assert_eq!(middle_ellipsis("short.zip", 20), "short.zip");
        let shown = middle_ellipsis("a-very-long-backup-name.tar.gz", 16);
        assert_eq!(shown.chars().count(), 16);
        assert!(
            shown.starts_with("a-very-") && shown.ends_with(".tar.gz"),
            "{shown}"
        );
        assert_eq!(middle_ellipsis("anything", 1), "…");
        assert_eq!(middle_ellipsis("anything", 0), "");
    }

    #[test]
    fn speeds_are_shown_per_second() {
        assert_eq!(speed(0.0), "0 B/s");
        assert!(speed(5.0 * 1024.0 * 1024.0).ends_with("MiB/s"));
        assert_eq!(speed(-3.0), "0 B/s");
    }
}
