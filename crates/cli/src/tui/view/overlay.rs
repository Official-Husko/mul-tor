//! Pop-ups drawn over the current screen.

use mul_tor_core::format_bytes;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, Paragraph, Wrap};

use super::widgets::{centered, panel, title};
use crate::tui::format::count;
use crate::tui::model::{Model, Overlay, Preflight, Screen};
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, model: &mut Model, theme: &Theme) {
    let screen = model.screen;
    let has_files = !model.files.basket.is_empty();
    let area = frame.area();
    let Model {
        overlay, providers, ..
    } = model;
    match overlay {
        None => {}
        Some(Overlay::Help) => help(frame, area, screen, theme),
        Some(Overlay::Quit) => quit(frame, area, screen, has_files, theme),
        Some(Overlay::Confirm(report)) => confirm(frame, area, report, theme),
        Some(Overlay::Presets(list)) => {
            let mut items = vec![ListItem::new(Line::from(vec![
                Span::styled("None", theme.text()),
                Span::styled("  choose by hand", theme.dim()),
            ]))];
            items.extend(providers.presets.iter().map(|preset| {
                ListItem::new(Line::from(vec![
                    Span::styled(preset.title.clone(), theme.accent()),
                    Span::styled(format!(" ({})", preset.stem), theme.dim()),
                    Span::styled(format!("  {} providers  ", preset.ids.len()), theme.dim()),
                    Span::styled(preset.description.clone(), theme.dim()),
                ]))
            }));
            let rect = centered(area, 72, (items.len() as u16 + 4).min(area.height));
            frame.render_widget(Clear, rect);
            let block = panel(
                title("Presets · Enter picks, Esc closes", theme),
                true,
                theme,
            );
            let inner = block.inner(rect);
            frame.render_widget(block, rect);
            frame.render_stateful_widget(
                List::new(items).highlight_style(theme.selected()),
                inner,
                list,
            );
        }
    }
}

/// A centred box. `footer` sits on the bottom border, so a box that is taller
/// than the window loses lines from its text but never its key hints.
fn popup(
    frame: &mut Frame,
    area: Rect,
    width: u16,
    lines: Vec<Line<'_>>,
    heading: &str,
    footer: Line<'_>,
    theme: &Theme,
) {
    let rect = centered(area, width, (lines.len() as u16 + 2).min(area.height));
    frame.render_widget(Clear, rect);
    let block = panel(title(heading, theme), true, theme).title_bottom(footer.centered());
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        rect,
    );
}

/// `Enter start   Esc back`, as it appears on a popup's border.
fn hints<'a>(pairs: &[(&'static str, &'static str)], theme: &Theme) -> Line<'a> {
    let mut spans = Vec::new();
    for (key, what) in pairs {
        spans.push(Span::styled(format!(" {key} "), theme.key()));
        spans.push(Span::styled(format!(" {what}  "), theme.dim()));
    }
    Line::from(spans)
}

fn key_line<'a>(key: &'static str, what: &'static str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {key:^7} "), theme.key()),
        Span::styled(format!("  {what}"), theme.text()),
    ])
}

fn help(frame: &mut Frame, area: Rect, screen: Screen, theme: &Theme) {
    let mut keys: Vec<(&'static str, &'static str)> = match screen {
        Screen::Files => vec![
            ("↑ ↓", "move"),
            ("Enter", "open a folder, or add a file"),
            ("Space", "add or remove the file; a folder adds its files"),
            ("a", "add every file in this folder"),
            (
                "/",
                "type a path (pasting or dropping paths works anywhere)",
            ),
            ("~ .  r", "home folder · hidden files · include sub-folders"),
            ("Tab", "switch between the browser and your list"),
            ("n", "next: choose providers"),
        ],
        Screen::Providers => vec![
            ("↑ ↓", "move"),
            ("Space", "tick or untick"),
            ("a  d", "tick all usable · untick all"),
            ("/", "filter by name"),
            ("p", "use a preset (a community's list of hosts)"),
            ("Enter", "review and upload"),
            ("Esc", "back to the files"),
        ],
        Screen::Upload => vec![
            ("↑ ↓", "scroll"),
            ("Esc", "stop the uploads"),
            ("Ctrl+C", "stop; twice to leave at once"),
        ],
        Screen::Results => vec![
            ("↑ ↓", "move"),
            ("c", "copy every link"),
            ("f", "copy the links in the preset's format"),
            ("b  n", "back to providers · start over"),
            ("q", "quit; the links are printed and saved"),
        ],
    };
    keys.extend([
        ("L", "show or hide the log"),
        ("?", "this help"),
        ("Ctrl+C", "quit"),
    ]);
    let lines = keys
        .into_iter()
        .map(|(key, what)| key_line(key, what, theme))
        .collect();
    popup(
        frame,
        area,
        76,
        lines,
        "Keys",
        hints(&[("any key", "close")], theme),
        theme,
    );
}

fn quit(frame: &mut Frame, area: Rect, screen: Screen, has_files: bool, theme: &Theme) {
    let warning = if screen == Screen::Upload {
        "Uploads are still running."
    } else if has_files {
        "Your list of files will be forgotten."
    } else {
        "Nothing is uploading."
    };
    let lines = vec![
        Line::styled("Quit Mul-Tor?", theme.accent()),
        Line::styled(warning, theme.dim()),
    ];
    popup(
        frame,
        area,
        44,
        lines,
        "Quit",
        hints(&[("y", "yes"), ("n", "no")], theme),
        theme,
    );
}

/// Names shown in the summary before the rest are folded into "and N more".
const LISTED_PROVIDERS: usize = 8;

fn confirm(frame: &mut Frame, area: Rect, report: &Preflight, theme: &Theme) {
    let mut names = report
        .providers
        .iter()
        .take(LISTED_PROVIDERS)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if report.providers.len() > LISTED_PROVIDERS {
        names.push_str(&format!(
            " ... and {} more",
            report.providers.len() - LISTED_PROVIDERS
        ));
    }
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!(
                    "{} ({})",
                    count(report.files, "file", "files"),
                    format_bytes(report.bytes)
                ),
                theme.accent(),
            ),
            Span::styled("  to  ", theme.dim()),
            Span::styled(
                count(report.providers.len(), "provider", "providers"),
                theme.accent(),
            ),
        ]),
        Line::styled(names, theme.dim()),
    ];
    if !report.warnings.is_empty() {
        lines.push(Line::raw(""));
        lines.extend(
            report
                .warnings
                .iter()
                .map(|w| Line::styled(format!("! {w}"), theme.warn())),
        );
    }
    if report.rejected > 0 {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!(
                "{} of {} uploads will fail before anything is sent:",
                report.rejected, report.uploads
            ),
            theme.error(),
        ));
        lines.extend(
            report
                .issues
                .iter()
                .map(|i| Line::styled(format!("  {i}"), theme.error())),
        );
        if report.unlisted() > 0 {
            lines.push(Line::styled(
                format!("  ...and {} more", report.unlisted()),
                theme.dim(),
            ));
        }
    }
    let footer = hints(&[("Enter", "start"), ("Esc", "back")], theme);
    popup(frame, area, 78, lines, "Ready to upload?", footer, theme);
}
