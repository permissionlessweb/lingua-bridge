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

    // Title with dseq info
    let dseq_info = app.bids_state.dseq
        .or(app.deployment_state.dseq)
        .map(|d| format!(" (DSeq: {})", d))
        .unwrap_or_default();
    let title = Paragraph::new(format!("Bid Selection{}", dseq_info))
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));

    frame.render_widget(title, layout[0]);

    // Bid table
    if app.bids_state.bids.is_empty() {
        let empty_msg = if app.bids_state.loading {
            "Fetching bids..."
        } else {
            "No bids available. Press 'r' to refresh."
        };
        let empty = Paragraph::new(empty_msg)
            .style(theme.text_dim_style())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(Span::styled(" Available Bids ", theme.primary_style()))
                    .borders(Borders::ALL)
                    .border_style(theme.primary_style()),
            );
        frame.render_widget(empty, layout[1]);
    } else {
        let rows: Vec<Row> = app.bids_state.bids.iter().enumerate().map(|(i, bid)| {
            let provider_short = if bid.provider.len() > 20 {
                format!("{}...", &bid.provider[..20])
            } else {
                bid.provider.clone()
            };
            let price = format!("{} {}", bid.price_amount, bid.price_denom);
            let style = if i == app.bids_state.selected_index {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_primary_style()
            };
            let marker = if i == app.bids_state.selected_index { ">" } else { " " };
            Row::new(vec![
                Cell::from(format!("{} {}", marker, provider_short)),
                Cell::from(price),
                Cell::from(bid.state.clone()),
            ])
            .style(style)
        }).collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(45),
                Constraint::Percentage(30),
                Constraint::Percentage(25),
            ],
        )
        .header(
            Row::new(vec!["Provider", "Price", "State"])
                .style(theme.primary_style().bold())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" Available Bids ({}) ", app.bids_state.bids.len()),
                    theme.primary_style(),
                ))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

        frame.render_widget(table, layout[1]);
    }
}
