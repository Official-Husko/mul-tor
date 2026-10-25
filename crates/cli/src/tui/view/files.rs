//! The file-choosing step: the folder browser and the list to upload.

use mul_tor_core::format_bytes;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

use super::widgets::{fit, panel, title};
use crate::tui::format::{count, middle_ellipsis};
use crate::tui::model::{Model, Pane};
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let [browser, basket] =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).areas(area);
    draw_browser(frame, browser, model, theme);
    draw_basket(frame, basket, model, theme);
}

fn draw_browser(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let files = &mut model.files;
    let focused = files.pane != Pane::Basket;
    let shown_dir = middle_ellipsis(
        &files.dir.to_string_lossy(),
        area.width.saturating_sub(14) as usize,
    );
    let flags = format!(
        " . hidden: {}  r recursive: {} ",
        if files.show_hidden { "on" } else { "off" },
        if files.recursive { "on" } else { "off" }
    );
    let block = panel(
        title(format!("Browse · {shown_dir}"), theme),
        focused,
        theme,
    )
    .title_bottom(Line::styled(flags, theme.dim()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (list_area, path_area) = if files.pane == Pane::Path {
        let [list, path] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(inner);
        (list, Some(path))
    } else {
        (inner, None)
    };

    if let Some(error) = &files.browse_error {
        let text = Paragraph::new(Line::styled(
            format!("Cannot read this folder: {error}"),
            theme.error(),
        ))
        .wrap(Wrap { trim: true });
        frame.render_widget(text, list_area);
    } else {
        let width = list_area.width as usize;
        let items: Vec<ListItem> = files
            .items
            .iter()
            .map(|item| {
                let mark = if !item.is_dir && files.contains(&item.path) {
                    "✓"
                } else {
                    " "
                };
                let (name, style) = if item.is_parent {
                    ("▲ ..".to_string(), theme.dim())
                } else if item.is_dir {
                    (format!("▸ {}/", item.name), theme.accent())
                } else {
                    (format!("  {}", item.name), theme.text())
                };
                let size = if item.is_dir {
                    String::new()
                } else {
                    format_bytes(item.size)
                };
                let name_width = width.saturating_sub(size.chars().count() + 4);
                ListItem::new(Line::from(vec![
                    Span::styled(mark, theme.ok()),
                    Span::raw(" "),
                    Span::styled(fit(&name, name_width), style),
                    Span::styled(format!("{size:>10}"), theme.dim()),
                ]))
            })
            .collect();
        let highlight = if files.pane == Pane::Browser {
            theme.selected()
        } else {
            theme.text()
        };
        frame.render_stateful_widget(
            List::new(items).highlight_style(highlight),
            list_area,
            &mut files.browser,
        );
    }

    if let Some(path_area) = path_area {
        let input = Paragraph::new(files.path_input.text().to_string()).block(panel(
            title("Path (Enter adds it, Esc cancels)", theme),
            true,
            theme,
        ));
        frame.render_widget(input, path_area);
        frame.set_cursor_position(Position::new(
            path_area.x + 1 + files.path_input.cursor_column(),
            path_area.y + 1,
        ));
    }
}

fn draw_basket(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let files = &mut model.files;
    let summary = format!(
        "To upload · {} · {}",
        count(files.basket.len(), "file", "files"),
        format_bytes(files.total_size())
    );
    let block = panel(title(summary, theme), files.pane == Pane::Basket, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if files.basket.is_empty() {
        let help = Paragraph::new(vec![
            Line::styled("Nothing chosen yet.", theme.dim()),
            Line::raw(""),
            Line::styled(
                "Press Space on a file, a to add every file here,",
                theme.dim(),
            ),
            Line::styled("or paste / drop paths onto this window.", theme.dim()),
        ])
        .wrap(Wrap { trim: true });
        frame.render_widget(help, inner);
        return;
    }
    let width = inner.width as usize;
    let items: Vec<ListItem> = files
        .basket
        .iter()
        .map(|entry| {
            let name = entry.path.file_name().map_or_else(
                || entry.path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            let size = format_bytes(entry.size);
            let name_width = width.saturating_sub(size.chars().count() + 2);
            ListItem::new(Line::from(vec![
                Span::styled(
                    fit(&middle_ellipsis(&name, name_width), name_width),
                    theme.text(),
                ),
                Span::styled(format!(" {size:>9}"), theme.dim()),
            ]))
        })
        .collect();
    let highlight = if files.pane == Pane::Basket {
        theme.selected()
    } else {
        theme.text()
    };
    frame.render_stateful_widget(
        List::new(items).highlight_style(highlight),
        inner,
        &mut files.basket_cursor,
    );
}
