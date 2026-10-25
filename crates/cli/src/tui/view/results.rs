//! The last step: what happened to each upload, and its link.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

use super::widgets::{fit, panel, title};
use crate::tui::format::{duration, middle_ellipsis};
use crate::tui::model::{JobState, Model};
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let counts = model.upload.counts();
    let heading = format!(
        "Results · {} uploaded · {} failed · {} cancelled · {}",
        counts.ok,
        counts.failed,
        counts.cancelled,
        duration(Some(model.upload.elapsed().as_secs()))
    );
    let [list_area, detail_area] = if area.height >= 16 {
        Layout::vertical([Constraint::Min(5), Constraint::Length(7)]).areas(area)
    } else {
        [area, Rect::default()]
    };

    let block = panel(title(heading, theme), true, theme);
    let inner = block.inner(list_area);
    frame.render_widget(block, list_area);
    let width = inner.width as usize;
    let items: Vec<ListItem> = model
        .upload
        .jobs
        .iter()
        .map(|job| {
            let (mark, mark_style, text, text_style) = match &job.state {
                JobState::Done { url } => ("✓", theme.ok(), url.clone(), theme.link()),
                JobState::Failed { error, .. } => {
                    ("✗", theme.error(), error.clone(), theme.error())
                }
                JobState::Cancelled => ("–", theme.warn(), "cancelled".to_string(), theme.warn()),
                _ => ("·", theme.dim(), "not finished".to_string(), theme.dim()),
            };
            let left = 16 + 1;
            let file_width = (width / 4).max(10);
            let rest = width.saturating_sub(left + file_width + 4);
            ListItem::new(Line::from(vec![
                Span::styled(mark, mark_style),
                Span::raw(" "),
                Span::styled(fit(job.provider, 16), theme.text()),
                Span::raw(" "),
                Span::styled(
                    fit(&middle_ellipsis(&job.file, file_width), file_width),
                    theme.dim(),
                ),
                Span::raw(" "),
                Span::styled(fit(&text, rest), text_style),
            ]))
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).highlight_style(theme.selected()),
        inner,
        &mut model.upload.list,
    );

    if detail_area.height > 0 {
        draw_detail(frame, detail_area, model, theme);
    }
}

fn draw_detail(frame: &mut Frame, area: Rect, model: &Model, theme: &Theme) {
    let block = panel(title("Selected", theme), false, theme);
    let Some(job) = model
        .upload
        .list
        .selected()
        .and_then(|index| model.upload.jobs.get(index))
    else {
        frame.render_widget(block, area);
        return;
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(job.provider, theme.accent()),
        Span::styled("  ·  ", theme.dim()),
        Span::styled(job.file.clone(), theme.text()),
    ])];
    match &job.state {
        JobState::Done { url } => {
            lines.push(Line::styled(url.clone(), theme.link()));
            if model.settings.save_links {
                lines.push(Line::styled(
                    format!(
                        "Saved to {}",
                        model.settings.links_dir.join("file_links.txt").display()
                    ),
                    theme.dim(),
                ));
            }
        }
        JobState::Failed { error, raw, kind } => {
            lines.push(Line::styled(error.clone(), theme.error()));
            lines.push(Line::styled(format!("kind: {kind}"), theme.dim()));
            if let Some(raw) = raw {
                let excerpt: String = raw.lines().take(2).collect::<Vec<_>>().join(" ");
                lines.push(Line::styled(format!("reply: {excerpt}"), theme.dim()));
            }
        }
        JobState::Cancelled => {
            lines.push(Line::styled("Stopped before it finished.", theme.warn()))
        }
        _ => {}
    }
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
