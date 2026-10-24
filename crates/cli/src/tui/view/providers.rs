//! The provider-choosing step: a checklist with filter, status and details.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use super::provider_details;
use super::widgets::{badge, fit, panel, title};
use crate::tui::model::{Model, Probe, ProviderRow};
use crate::tui::theme::Theme;

/// Wide enough for the details panel next to the list.
const DETAILS_MIN_WIDTH: u16 = 96;
const DETAILS_WIDTH: u16 = 40;

pub fn draw(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let (list_area, details) = if area.width >= DETAILS_MIN_WIDTH {
        let [list, details] =
            Layout::horizontal([Constraint::Min(40), Constraint::Length(DETAILS_WIDTH)])
                .areas(area);
        (list, Some(details))
    } else {
        (area, None)
    };
    draw_list(frame, list_area, model, theme);
    if let Some(details) = details {
        provider_details::draw(frame, details, model, theme);
    }
}

fn draw_list(frame: &mut Frame, area: Rect, model: &mut Model, theme: &Theme) {
    let tick = model.tick;
    let providers = &mut model.providers;
    let heading = format!(
        "Providers · {} of {} ticked{}",
        providers.selected.len(),
        providers.rows.len(),
        if providers.probing {
            format!(" · checking {}", spinner(tick))
        } else {
            String::new()
        }
    );
    let block = panel(title(heading, theme), !providers.filtering, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [filter_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
    let filter_style = if providers.filtering {
        theme.accent()
    } else {
        theme.dim()
    };
    let hint = if providers.filter.is_empty() && !providers.filtering {
        "type / to filter"
    } else {
        ""
    };
    let filter_line = Line::from(vec![
        Span::styled("Filter: ", filter_style),
        Span::styled(providers.filter.text().to_string(), theme.text()),
        Span::styled(hint, theme.dim()),
    ]);
    frame.render_widget(Paragraph::new(filter_line), filter_area);
    if providers.filtering {
        frame.set_cursor_position(Position::new(
            filter_area.x + 8 + providers.filter.cursor_column(),
            filter_area.y,
        ));
    }

    let visible = providers.visible();
    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled("No provider matches.", theme.dim())),
            list_area,
        );
        return;
    }
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&index| {
            row_line(
                &providers.rows[index],
                providers.is_selected(&providers.rows[index]),
                theme,
            )
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items).highlight_style(theme.selected()),
        list_area,
        &mut providers.list,
    );
}

fn row_line<'a>(row: &ProviderRow, ticked: bool, theme: &Theme) -> ListItem<'a> {
    let info = row.provider.info();
    let (box_text, box_style) = match (ticked, row.usable) {
        (true, _) => ("[x]", theme.ok()),
        (false, true) => ("[ ]", theme.text()),
        (false, false) => ("[-]", theme.dim()),
    };
    let name_style = if row.usable {
        theme.text()
    } else {
        theme.dim()
    };
    let limit = info
        .max_size
        .map_or_else(|| "no limit".to_string(), |l| l.to_string());
    let probe = match &row.probe {
        Probe::Online => badge("● online ", theme.ok()),
        Probe::Offline(_) => badge("● offline", theme.error()),
        Probe::Unknown => badge("○ ?      ", theme.dim()),
    };
    let reliability = info.reliability;
    let (status, status_style) = match reliability.label() {
        "working" => ("working", theme.ok()),
        "degraded" => ("degraded", theme.warn()),
        "broken" => ("broken", theme.error()),
        _ => ("unverified", theme.dim()),
    };
    let mut spans = vec![
        Span::styled(box_text, box_style),
        Span::raw(" "),
        Span::styled(fit(info.name, 16), name_style),
        Span::styled(format!("{limit:>9}  "), theme.dim()),
        probe,
        Span::raw("  "),
        Span::styled(fit(status, 10), status_style),
    ];
    if !row.usable {
        spans.push(Span::styled(" needs account", theme.warn()));
    }
    ListItem::new(Line::from(spans))
}

/// A braille spinner frame; one step per tick.
pub fn spinner(tick: u64) -> char {
    const FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
    FRAMES[(tick / 2) as usize % FRAMES.len()]
}
