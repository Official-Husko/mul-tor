//! The `{placeholder}` link templates presets use.

/// Substitute `{name}` placeholders from `values`, like Python's `str.format`
/// restricted to named fields: `{{` and `}}` are literal braces, and a
/// placeholder that is unknown (or unterminated) is left as written.
pub fn format_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(pos) = rest.find(['{', '}']) {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        if let Some(after) = tail.strip_prefix("{{") {
            out.push('{');
            rest = after;
        } else if let Some(after) = tail.strip_prefix("}}") {
            out.push('}');
            rest = after;
        } else if let Some(inner) = tail.strip_prefix('{') {
            match inner.find('}') {
                Some(end) => {
                    let key = &inner[..end];
                    match values.iter().find(|(name, _)| *name == key) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(&tail[..end + 2]),
                    }
                    rest = &inner[end + 1..];
                }
                None => {
                    out.push('{');
                    rest = inner;
                }
            }
        } else {
            // A lone `}` is kept as written.
            out.push('}');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// The placeholders a link template can use. This is the only list, so every
/// front-end fills a preset's template the same way.
pub fn template_fields<'a>(
    provider: &'a str,
    file_name: &'a str,
    url: &'a str,
    when: &'a str,
) -> [(&'static str, &'a str); 6] {
    [
        ("status", "ok"),
        ("file_name", file_name),
        ("file_url", url),
        ("provider_name", provider),
        // Older spelling, still used by shared preset files.
        ("site_name", provider),
        ("date_and_time", when),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_substituted() {
        let out = format_template(
            "[URL='{file_url}']{provider_name}[/URL]",
            &[("file_url", "https://x/y"), ("provider_name", "Pixeldrain")],
        );
        assert_eq!(out, "[URL='https://x/y']Pixeldrain[/URL]");
    }

    #[test]
    fn doubled_braces_are_literal() {
        assert_eq!(format_template("{{{a}}}", &[("a", "1")]), "{1}");
        assert_eq!(format_template("{{a}}", &[("a", "1")]), "{a}");
    }

    #[test]
    fn unknown_and_unterminated_placeholders_are_kept() {
        assert_eq!(format_template("{nope} {a", &[("a", "1")]), "{nope} {a");
        assert_eq!(format_template("}", &[]), "}");
    }

    #[test]
    fn non_ascii_text_survives() {
        assert_eq!(format_template("日本 {a} é", &[("a", "ü")]), "日本 ü é");
        // Multi-byte characters inside an unknown placeholder must not eat what follows.
        assert_eq!(format_template("{é}x{a}", &[("a", "1")]), "{é}x1");
        assert_eq!(format_template("{日本}", &[("日本", "ok")]), "ok");
    }
}
