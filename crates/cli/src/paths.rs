//! Turning what a user types or drags into a terminal into paths.
//!
//! Dropping a file onto a terminal pastes its path: quoted if it contains
//! spaces (Windows, macOS Terminal), or backslash-escaped (most Linux
//! terminals). Several dropped files arrive as one line.

use std::path::PathBuf;

/// Split `line` into paths, understanding quotes and (on Unix) backslash escapes.
pub fn parse_paths(line: &str) -> Vec<PathBuf> {
    split_words(line, cfg!(unix))
        .into_iter()
        .map(|word| expand_home(&word))
        .collect()
}

fn split_words(line: &str, backslash_escapes: bool) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                in_word = true;
            }
            (None, '\\') if backslash_escapes => {
                // Only unescape characters a shell would escape; leave other
                // backslashes alone.
                match chars.peek() {
                    Some(&next)
                        if next.is_whitespace() || "\"'\\()[]{}&;$`!#~*?<>|".contains(next) =>
                    {
                        current.push(next);
                        chars.next();
                    }
                    _ => current.push('\\'),
                }
                in_word = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            (None, c) => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

fn expand_home(word: &str) -> PathBuf {
    if (word == "~" || word.starts_with("~/") || word.starts_with("~\\"))
        && let Some(dirs) = directories::BaseDirs::new()
    {
        return dirs
            .home_dir()
            .join(word[1..].trim_start_matches(['/', '\\']));
    }
    PathBuf::from(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(line: &str) -> Vec<String> {
        parse_paths(line)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn plain_paths_split_on_whitespace() {
        assert_eq!(paths("/a/b.zip /c/d.zip"), ["/a/b.zip", "/c/d.zip"]);
        assert_eq!(paths("  one  "), ["one"]);
        assert!(paths("   ").is_empty());
    }

    #[test]
    fn quoted_paths_may_contain_spaces() {
        assert_eq!(
            paths(r#""C:\My Files\a b.zip" 'x y' z"#),
            [r"C:\My Files\a b.zip", "x y", "z"]
        );
    }

    #[test]
    fn an_empty_quoted_word_is_kept_as_a_word() {
        assert_eq!(paths(r#"a "" b"#), ["a", "", "b"]);
    }

    #[cfg(unix)]
    #[test]
    fn unix_backslash_escapes_are_undone() {
        assert_eq!(
            paths(r"/tmp/my\ file\ (1).zip other"),
            ["/tmp/my file (1).zip", "other"]
        );
        // A backslash before an ordinary character stays.
        assert_eq!(paths(r"/tmp/a\b"), [r"/tmp/a\b"]);
    }

    #[test]
    fn windows_style_backslashes_are_not_escapes() {
        assert_eq!(
            split_words(r"C:\Users\me\a.zip", false),
            [r"C:\Users\me\a.zip"]
        );
    }

    #[test]
    fn unterminated_quotes_take_the_rest_of_the_line() {
        assert_eq!(paths(r#""half open path"#), ["half open path"]);
    }

    #[test]
    fn tilde_expands_to_the_home_directory() {
        let expanded = parse_paths("~/Downloads/a.zip").remove(0);
        assert!(expanded.ends_with("Downloads/a.zip"));
        assert!(!expanded.to_string_lossy().starts_with('~'));
        // A tilde in the middle of a name is just a character.
        assert_eq!(paths("a~b"), ["a~b"]);
    }
}
