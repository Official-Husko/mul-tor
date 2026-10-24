//! The panel describing the provider under the cursor.

use mul_tor_core::{Auth, Reliability};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::widgets::{panel, title};
use crate::tui::model::{Model, Probe};
use crate::tui::theme::Theme;

pub fn draw(frame: &mut Frame, area: Rect, model: &Model, theme: &Theme) {
    let block = panel(title("Details", theme), false, theme);
    let Some(row) = model.providers.current() else {
        frame.render_widget(block, area);
        return;
    };
    let info = row.provider.info();
    let label = |text: &'static str| Span::styled(format!("{text:<10}"), theme.dim());

    let mut lines = vec![
        Line::styled(info.name, theme.accent()),
        Line::styled(info.homepage, theme.dim()),
        Line::raw(""),
        Line::from(vec![
            label("Limit"),
            Span::raw(
                info.max_size
                    .map_or_else(|| "none known".to_string(), |l| l.to_string()),
            ),
        ]),
    ];

    let (account, account_style) = match (info.auth, row.usable) {
        (Auth::None, _) => ("not needed", theme.text()),
        (Auth::Optional, _) => ("optional", theme.text()),
        (Auth::Required, true) => ("set up", theme.ok()),
        (Auth::Required, false) => ("needed, not set up", theme.warn()),
    };
    lines.push(Line::from(vec![
        label("Account"),
        Span::styled(account, account_style),
    ]));
    if let (Auth::Required, Some(url)) = (info.auth, info.credentials_url) {
        lines.push(Line::from(vec![label(""), Span::styled(url, theme.dim())]));
    }

    let (state, state_style) = match &row.probe {
        Probe::Online => ("online".to_string(), theme.ok()),
        Probe::Offline(reason) => (format!("offline ({reason})"), theme.error()),
        Probe::Unknown => ("not checked".to_string(), theme.dim()),
    };
    lines.push(Line::from(vec![
        label("Reachable"),
        Span::styled(state, state_style),
    ]));

    let (reliability, reliability_style) = match info.reliability {
        Reliability::Unverified => ("unverified".to_string(), theme.dim()),
        Reliability::Working { checked } => (format!("working (seen {checked})"), theme.ok()),
        Reliability::Degraded(_) => ("degraded".to_string(), theme.warn()),
        Reliability::Broken(_) => ("broken".to_string(), theme.error()),
    };
    lines.push(Line::from(vec![
        label("Status"),
        Span::styled(reliability, reliability_style),
    ]));
    if let Some(note) = info.reliability.note().filter(|_| {
        matches!(
            info.reliability,
            Reliability::Degraded(_) | Reliability::Broken(_)
        )
    }) {
        lines.push(Line::styled(note, theme.warn()));
    }
    if matches!(info.reliability, Reliability::Unverified) {
        lines.push(Line::styled(
            "Nobody has run this against the live service yet.",
            theme.dim(),
        ));
    }
    if !info.blocked_extensions.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            label("Refuses"),
            Span::raw(info.blocked_extensions.join(" ")),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
