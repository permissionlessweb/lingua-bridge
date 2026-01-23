use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::{App, DeploymentStatus};
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
    let title = Paragraph::new("Deployed Bots")
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));
    frame.render_widget(title, layout[0]);

    if app.deployments_state.deployments.is_empty() {
        // Empty state
        let empty_lines = vec![
            Line::from(""),
            Line::from(Span::styled("No deployments found", theme.text_dim_style())),
            Line::from(""),
            Line::from(Span::styled("Press 2 to start a new deployment", theme.text_primary_style())),
            Line::from(Span::styled("or press r to refresh", theme.text_dim_style())),
        ];
        let empty = Paragraph::new(empty_lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(Span::styled(" Dashboard ", theme.primary_style()))
                    .borders(Borders::ALL)
                    .border_style(theme.primary_style()),
            );
        frame.render_widget(empty, layout[1]);
        return;
    }

    // Content: left = deployment list, right = details
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(layout[1]);

    // Left panel: deployment list
    let mut list_lines = Vec::new();
    for (i, dep) in app.deployments_state.deployments.iter().enumerate() {
        let selected = i == app.deployments_state.selected_index;
        let marker = if selected { ">" } else { " " };
        let status_style = match dep.status {
            DeploymentStatus::Active => Style::default().fg(theme.success),
            DeploymentStatus::Terminated => theme.text_dim_style(),
            DeploymentStatus::Failed => Style::default().fg(theme.error),
            DeploymentStatus::Unknown => Style::default().fg(theme.info),
        };
        let name_style = if selected {
            Style::default().fg(theme.primary).bold()
        } else {
            theme.text_primary_style()
        };

        list_lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), name_style),
            Span::styled(&dep.name, name_style),
            Span::styled(format!(" [{}]", dep.status.as_str()), status_style),
        ]));
        list_lines.push(Line::from(Span::styled(
            format!("  DSeq: {} | {}", dep.dseq, dep.created_at),
            theme.text_dim_style(),
        )));
    }

    let list_panel = Paragraph::new(list_lines)
        .block(
            Block::default()
                .title(Span::styled(" Deployments ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(list_panel, content_layout[0]);

    // Right panel: details for selected deployment
    let mut detail_lines = Vec::new();
    if let Some(dep) = app.deployments_state.deployments.get(app.deployments_state.selected_index) {
        detail_lines.push(Line::from(Span::styled(&dep.name, theme.text_primary_style().bold())));
        detail_lines.push(Line::from(""));

        let status_style = match dep.status {
            DeploymentStatus::Active => Style::default().fg(theme.success),
            DeploymentStatus::Terminated => theme.text_dim_style(),
            DeploymentStatus::Failed => Style::default().fg(theme.error),
            DeploymentStatus::Unknown => Style::default().fg(theme.info),
        };
        detail_lines.push(Line::from(vec![
            Span::styled("Status: ", theme.text_dim_style()),
            Span::styled(dep.status.as_str(), status_style),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("DSeq: ", theme.text_dim_style()),
            Span::styled(dep.dseq.to_string(), theme.text_primary_style()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Created: ", theme.text_dim_style()),
            Span::styled(&dep.created_at, theme.text_primary_style()),
        ]));

        if !dep.services.is_empty() {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled("Services", theme.text_primary_style().bold())));
            for svc in &dep.services {
                let uri_text = svc.uri.as_deref().unwrap_or("N/A");
                detail_lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", svc.name), theme.text_dim_style()),
                    Span::styled(uri_text, Style::default().fg(theme.info)),
                ]));
            }
        }
    }

    let detail_panel = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .title(Span::styled(" Details ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(detail_panel, content_layout[1]);
}
