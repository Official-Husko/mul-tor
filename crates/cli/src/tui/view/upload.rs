//! The upload step: one overall bar, one line per upload.

use mul_tor_core::format_bytes;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use super::widgets::{bar, fit, panel, title};
use crate::tui::format::{count, duration, middle_ellipsis, percent, speed};
use crate::tui::model::{JobState, Model};
use crate::tui::theme::Theme;

const PROVIDER_WIDTH: usize = 14;
const BAR_WIDTH: u16 = 14;
/// The space and `100%` between the bar and the note.
const BAR_EXTRAS: usize = 6;
const MIN_FILE_WIDTH: usize = 16;
const MIN_NOTE_WIDTH: usize = 8;
/// Wide enough for "waiting for reply".
const MAX_NOTE_WIDTH: usize = 18;

pub fn draw(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let [overall, jobs] = Layout::vertical([Constraint::Length(4), Constraint::Min(3)]).areas(area);
    draw_overall(frame, overall, model, theme);
    draw_jobs(frame, jobs, model, theme);
}

fn draw_overall(frame: &mut Frame, area: Rect, model: &Model, theme: &Theme) {
    let upload = &model.upload;
    let total_jobs = upload.jobs.len();
    let heading = format!(
        "{} · {} of {} done",
        if upload.cancelling {
            "Cancelling"
        } else {
            "Uploading"
        },
        upload.finished_jobs(),
        count(total_jobs, "upload", "uploads")
    );
    let block = panel(title(heading, theme), true, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (done, total) = upload.bytes();
    let fraction = if total == 0 {
        0.0
    } else {
        done as f64 / total as f64
    };
    let bar_width = inner.width.saturating_sub(6);
    let mut spans = bar(fraction, bar_width, theme);
    spans.push(Span::styled(
        format!(" {:>3}%", percent(done, total)),
        theme.accent(),
    ));
    let stats = Line::from(vec![
        Span::styled(
            format!("{} / {}", format_bytes(done), format_bytes(total)),
            theme.text(),
        ),
        Span::styled("  ·  ", theme.dim()),
        Span::styled(speed(upload.speed()), theme.text()),
        Span::styled("  ·  left ", theme.dim()),
        Span::styled(duration(upload.eta_seconds()), theme.text()),
        Span::styled("  ·  elapsed ", theme.dim()),
        Span::styled(duration(Some(upload.elapsed().as_secs())), theme.text()),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::from(spans), stats]), inner);
}

fn draw_jobs(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let block = panel(title("Uploads", theme), false, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let items: Vec<ListItem> = model
        .upload
        .jobs
        .iter()
        .map(|job| job_line(job.provider, &job.file, &job.state, width, theme))
        .collect();
    frame.render_stateful_widget(
        List::new(items).highlight_style(theme.selected()),
        inner,
        &mut model.upload.list,
    );
}

/// `Provider   file name …   ████░░░  42%  3.1 MiB/s`
fn job_line<'a>(
    provider: &str,
    file: &str,
    state: &JobState,
    width: usize,
    theme: &Theme,
) -> ListItem<'a> {
    // The bar and percentage are fixed; the note after them (speed, or what
    // the provider is doing) gets what is left once the file name has its
    // minimum, up to the widest note.
    let note_width = width
        .saturating_sub(PROVIDER_WIDTH + BAR_WIDTH as usize + BAR_EXTRAS + 3 + MIN_FILE_WIDTH)
        .clamp(MIN_NOTE_WIDTH, MAX_NOTE_WIDTH);
    let right_width = BAR_WIDTH as usize + BAR_EXTRAS + note_width;
    let file_width = width
        .saturating_sub(PROVIDER_WIDTH + right_width + 3)
        .max(MIN_FILE_WIDTH / 2);
    let mut spans = vec![
        Span::styled(fit(provider, PROVIDER_WIDTH), theme.accent()),
        Span::raw(" "),
        Span::styled(
            fit(&middle_ellipsis(file, file_width), file_width),
            theme.text(),
        ),
        Span::raw(" "),
    ];
    match state {
        JobState::Queued => spans.push(Span::styled(fit("waiting", right_width), theme.dim())),
        JobState::Running {
            sent,
            total,
            message,
            speed: rate,
            ..
        } => {
            let fraction = if *total == 0 {
                0.0
            } else {
                *sent as f64 / *total as f64
            };
            spans.extend(bar(fraction, BAR_WIDTH, theme));
            spans.push(Span::styled(
                format!(" {:>3}% ", percent(*sent, *total)),
                theme.accent(),
            ));
            // Everything is sent but the provider has not answered yet: a
            // speed would be a lie, and the bar sits at 100%.
            let note = if !message.is_empty() {
                message.clone()
            } else if *total > 0 && sent >= total {
                "waiting for reply".to_string()
            } else {
                speed(*rate)
            };
            spans.push(Span::styled(fit(&note, note_width), theme.dim()));
        }
        JobState::Done { .. } => {
            spans.push(Span::styled(fit("✓ uploaded", right_width), theme.ok()))
        }
        JobState::Failed { error, .. } => {
            spans.push(Span::styled(
                fit(&format!("✗ {error}"), right_width),
                theme.error(),
            ));
        }
        JobState::Cancelled => {
            spans.push(Span::styled(fit("– cancelled", right_width), theme.warn()))
        }
    }
    ListItem::new(Line::from(spans))
}
