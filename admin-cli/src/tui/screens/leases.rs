use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::App;
use crate::tui::theme::AkashTheme;

pub fn render(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Lease Management")
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));

    frame.render_widget(title, layout[0]);

    // Content area
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);

    // Left panel - Lease table
    if app.leases_state.leases.is_empty() {
        let empty_msg = if app.leases_state.loading {
            "Fetching leases..."
        } else {
            "No active leases. Press 'r' to refresh."
        };
        let empty = Paragraph::new(empty_msg)
            .style(theme.text_dim_style())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(Span::styled(" Active Leases ", theme.primary_style()))
                    .borders(Borders::ALL)
                    .border_style(theme.primary_style()),
            );
        frame.render_widget(empty, content_layout[0]);
    } else {
        let rows: Vec<Row> = app.leases_state.leases.iter().enumerate().map(|(i, lease)| {
            let provider_short = if lease.provider.len() > 16 {
                format!("{}...", &lease.provider[..16])
            } else {
                lease.provider.clone()
            };
            let style = if i == app.leases_state.selected_index {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_primary_style()
            };
            let marker = if i == app.leases_state.selected_index { ">" } else { " " };
            Row::new(vec![
                Cell::from(format!("{} {}", marker, lease.dseq)),
                Cell::from(provider_short),
                Cell::from(lease.state.clone()),
            ])
            .style(style)
        }).collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ],
        )
        .header(
            Row::new(vec!["DSeq", "Provider", "State"])
                .style(theme.primary_style().bold())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" Active Leases ({}) ", app.leases_state.leases.len()),
                    theme.primary_style(),
                ))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

        frame.render_widget(table, content_layout[0]);
    }

    // Right panel - Details and logs
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(content_layout[1]);

    // Lease details
    let selected_lease = app.leases_state.leases.get(app.leases_state.selected_index);
    let mut detail_lines = vec![
        Line::from(Span::styled("Selected Lease", theme.text_primary_style().bold())),
        Line::from(""),
    ];

    if let Some(lease) = selected_lease {
        detail_lines.push(Line::from(vec![
            Span::styled("  DSeq: ", theme.text_dim_style()),
            Span::styled(format!("{}", lease.dseq), theme.text_primary_style()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("  GSeq: ", theme.text_dim_style()),
            Span::styled(format!("{}", lease.gseq), theme.text_primary_style()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("  Provider: ", theme.text_dim_style()),
            Span::styled(&*lease.provider, theme.text_primary_style()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("  Price: ", theme.text_dim_style()),
            Span::styled(format!("{} {}", lease.price_amount, lease.price_denom), theme.text_primary_style()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("  State: ", theme.text_dim_style()),
            Span::styled(&*lease.state, Style::default().fg(theme.success)),
        ]));
    } else {
        detail_lines.push(Line::from(Span::styled("  None selected", theme.text_dim_style())));
    }

    if !app.leases_state.service_uris.is_empty() {
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled("Service URIs", theme.text_primary_style().bold())));
        for uri in &app.leases_state.service_uris {
            detail_lines.push(Line::from(Span::styled(format!("  {}", uri), Style::default().fg(theme.info))));
        }
    }

    let details = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .title(Span::styled(" Details ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

    frame.render_widget(details, right_layout[0]);

    // Logs
    app.leases_state.log_viewer.render(frame, theme, right_layout[1]);
}
