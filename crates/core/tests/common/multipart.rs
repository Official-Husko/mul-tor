//! Reading `multipart/form-data` bodies back, to check what was sent.

#[derive(Clone, Debug)]
pub struct FormPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    /// The `Content-Disposition` line exactly as sent.
    pub disposition: String,
    pub data: Vec<u8>,
}

impl FormPart {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

/// Find a part by field name.
pub fn field<'a>(parts: &'a [FormPart], name: &str) -> &'a FormPart {
    parts.iter().find(|p| p.name == name).unwrap_or_else(|| {
        panic!(
            "no multipart field `{name}`; got {:?}",
            parts.iter().map(|p| &p.name).collect::<Vec<_>>()
        )
    })
}

pub fn text_field(parts: &[FormPart], name: &str) -> String {
    field(parts, name).text()
}

pub fn parse_multipart(content_type: &str, body: &[u8]) -> Vec<FormPart> {
    let boundary = content_type
        .split(';')
        .filter_map(|p| p.trim().strip_prefix("boundary="))
        .next()
        .unwrap_or_else(|| panic!("no boundary in `{content_type}`"))
        .trim_matches('"');
    let delimiter = format!("--{boundary}").into_bytes();

    let mut parts = Vec::new();
    let mut rest = body;
    // Skip the preamble up to the first delimiter.
    let Some(start) = find(rest, &delimiter) else {
        return parts;
    };
    rest = &rest[start + delimiter.len()..];
    loop {
        if rest.starts_with(b"--") {
            break; // closing delimiter
        }
        rest = rest.strip_prefix(b"\r\n").unwrap_or(rest);
        let Some(header_end) = find(rest, b"\r\n\r\n") else {
            break;
        };
        let headers = String::from_utf8_lossy(&rest[..header_end]).into_owned();
        let after_headers = &rest[header_end + 4..];
        let mut next = delimiter.clone();
        next.splice(0..0, *b"\r\n");
        let Some(data_end) = find(after_headers, &next) else {
            break;
        };
        let data = after_headers[..data_end].to_vec();
        rest = &after_headers[data_end + next.len()..];

        let disposition = headers
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("content-disposition"))
            .unwrap_or("")
            .to_string();
        let content_type = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Type: "))
            .map(str::to_string);
        parts.push(FormPart {
            name: quoted(&disposition, "name").unwrap_or_default(),
            filename: quoted(&disposition, "filename"),
            content_type,
            disposition,
            data,
        });
    }
    parts
}

fn quoted(disposition: &str, key: &str) -> Option<String> {
    // Match ` name="` / `; filename="` without confusing `name` and `filename`.
    let needle = format!("{key}=\"");
    let mut search = disposition;
    while let Some(pos) = search.find(&needle) {
        let preceded_ok = pos == 0 || matches!(search.as_bytes()[pos - 1], b' ' | b';');
        let value = &search[pos + needle.len()..];
        if preceded_ok {
            // Read up to the first unescaped quote, undoing `\"` and `\\`.
            let mut out = String::new();
            let mut chars = value.chars();
            while let Some(c) = chars.next() {
                match c {
                    '"' => return Some(out),
                    '\\' => out.extend(chars.next()),
                    c => out.push(c),
                }
            }
            return None;
        }
        search = &search[pos + needle.len()..];
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
