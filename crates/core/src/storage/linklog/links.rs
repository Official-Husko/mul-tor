//! The text a user copies after a run: one link per line.

use super::template::{format_template, template_fields};

/// A finished upload, as far as copying its link is concerned.
#[derive(Clone, Copy, Debug)]
pub struct FinishedLink<'a> {
    pub provider: &'a str,
    pub file_name: &'a str,
    pub url: &'a str,
}

/// One line per link: the bare URL, or `template` filled in (a preset's
/// `link-format`, with `date_and_time` left empty).
pub fn render_links<'a>(
    links: impl IntoIterator<Item = FinishedLink<'a>>,
    template: Option<&str>,
) -> String {
    links
        .into_iter()
        .map(|link| match template {
            Some(template) => format_template(
                template,
                &template_fields(link.provider, link.file_name, link.url, ""),
            ),
            None => link.url.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links() -> Vec<FinishedLink<'static>> {
        vec![
            FinishedLink {
                provider: "Catbox",
                file_name: "a.zip",
                url: "https://c/1",
            },
            FinishedLink {
                provider: "Keep",
                file_name: "a.zip",
                url: "https://k/2",
            },
        ]
    }

    #[test]
    fn without_a_template_the_links_are_bare_urls() {
        assert_eq!(render_links(links(), None), "https://c/1\nhttps://k/2");
    }

    #[test]
    fn a_template_is_filled_in_for_every_link() {
        assert_eq!(
            render_links(links(), Some("[URL='{file_url}']{provider_name}[/URL]")),
            "[URL='https://c/1']Catbox[/URL]\n[URL='https://k/2']Keep[/URL]"
        );
        assert_eq!(
            render_links(links(), Some("{site_name}: {file_name}")),
            "Catbox: a.zip\nKeep: a.zip"
        );
    }

    #[test]
    fn nothing_finished_means_no_text() {
        assert_eq!(render_links([], None), "");
    }
}
