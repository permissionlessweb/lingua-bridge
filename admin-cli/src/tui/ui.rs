use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::tui::app::{App, MainTab, Screen};
use crate::tui::screens;
use crate::tui::theme::AkashTheme;

/// Render the current application state
pub fn render(frame: &mut Frame, app: &App) {
    let theme = AkashTheme::default();

    // Main layout: header, tab bar, content, footer
    let show_tabs = !app.show_splash;
    let main_layout = if show_tabs {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(1), // Tab bar
                Constraint::Min(1),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(frame.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(0), // No tab bar on splash
                Constraint::Min(1),    // Content
                Constraint::Length(3), // Footer
            ])
            .split(frame.area())
    };

    // Render header
    render_header(frame, &theme, app, main_layout[0]);

    // Render tab bar (only after splash)
    if show_tabs {
        render_tab_bar(frame, &theme, app, main_layout[1]);
    }

    // Render current screen content
    render_screen(frame, &theme, app, main_layout[2]);

    // Render footer
    render_footer(frame, &theme, app, main_layout[3]);

    // Render spinner overlay if active
    if app.spinner.spinning {
        render_spinner(frame, &theme, app);
    }

    // Render popup overlay if visible
    if let Some(popup) = &app.popup {
        if popup.visible {
            render_popup(frame, &theme, app);
        }
    }
}

fn render_header(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let status = if let Some((ref msg, is_error)) = app.status_message {
        let style = if is_error {
            Style::default().fg(theme.error)
        } else {
            Style::default().fg(theme.success)
        };
        Span::styled(format!(" │ {}", msg), style)
    } else {
        Span::raw("")
    };

    let title_line = Line::from(vec![
        Span::styled("LinguaBridge Admin TUI", theme.primary_style().bold()),
        status,
    ]);

    let header = Paragraph::new(title_line)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

    frame.render_widget(header, area);
}

fn render_tab_bar(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let tabs = vec![
        ("1:Bots", MainTab::Deployments),
        ("2:Deploy", MainTab::Deploy),
        ("3:Wallet", MainTab::Wallet),
    ];

    let mut spans = Vec::new();
    spans.push(Span::styled(" ", theme.text_dim_style()));

    for (i, (label, tab)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", theme.text_dim_style()));
        }
        let style = if *tab == app.main_tab {
            Style::default().fg(theme.primary).bold()
        } else {
            theme.text_dim_style()
        };
        spans.push(Span::styled(*label, style));
    }

    // Show deploy step indicator when on Deploy tab
    if app.main_tab == MainTab::Deploy {
        let step_name = match app.deploy_step {
            crate::tui::app::DeployStep::SdlConfig => "SDL",
            crate::tui::app::DeployStep::FeeGrant => "Fee",
            crate::tui::app::DeployStep::Submit => "Submit",
            crate::tui::app::DeployStep::Bids => "Bids",
            crate::tui::app::DeployStep::Leases => "Leases",
            crate::tui::app::DeployStep::DiscordConfig => "Discord",
        };
        spans.push(Span::styled(format!("  → {}", step_name), Style::default().fg(theme.info)));
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

fn render_screen(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    match app.current_screen {
        Screen::Splash => screens::render_splash(frame, theme, area),
        Screen::Wallet => screens::render_wallet(frame, theme, app, area),
        Screen::FeeGrant => screens::render_fee_grant(frame, theme, app, area),
        Screen::Deployment => screens::render_deployment(frame, theme, app, area),
        Screen::Bids => screens::render_bids(frame, theme, app, area),
        Screen::Leases => screens::render_leases(frame, theme, app, area),
        Screen::DiscordConfig => screens::render_discord_config(frame, theme, app, area),
        Screen::Deployments => screens::render_deployments(frame, theme, app, area),
    }
}

fn render_footer(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let mode_text = match app.input_mode {
        crate::tui::input::InputMode::Normal => "NORMAL",
        crate::tui::input::InputMode::Insert => "INSERT",
        crate::tui::input::InputMode::Command => "COMMAND",
    };

    let mode_style = theme.mode_style(app.input_mode);

    let screen_text = match app.current_screen {
        Screen::Splash => "SPLASH",
        Screen::Wallet => "WALLET",
        Screen::FeeGrant => "FEE GRANT",
        Screen::Deployment => "DEPLOYMENT",
        Screen::Bids => "BIDS",
        Screen::Leases => "LEASES",
        Screen::DiscordConfig => "DISCORD CONFIG",
        Screen::Deployments => "DEPLOYED BOTS",
    };

    let help_text = match app.current_screen {
        Screen::Splash => "Press any key to continue",
        Screen::Wallet => "g: Gen | i: Import | c: Copy | s: Save | l: Load | r: Balance",
        Screen::FeeGrant => "r: Request | c: Check Grants | b: Balance | Tab/BackTab: Nav",
        Screen::Deployment => "v: Panel | i: Edit | j/k: Nav | g: GPU | d: Deploy",
        Screen::Bids => "j/k: Navigate | Enter: Accept | r: Refresh",
        Screen::Leases => "j/k: Navigate | l: Logs | r: Refresh",
        Screen::DiscordConfig => "i: Edit | j/k: Field | x/X: Clear | u: URL | t: Test | n/p: Guide",
        Screen::Deployments => "j/k: Navigate | r: Refresh | l: Logs | 2: New Deploy",
    };

    let footer_line = Line::from(vec![
        Span::styled(format!(" {} ", mode_text), mode_style.bold()),
        Span::styled(" │ ", theme.text_dim_style()),
        Span::styled(screen_text, theme.text_primary_style()),
        Span::styled(" │ ", theme.text_dim_style()),
        Span::styled(help_text, theme.text_dim_style()),
    ]);

    let footer = Paragraph::new(footer_line)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

    frame.render_widget(footer, area);
}

fn render_spinner(frame: &mut Frame, theme: &AkashTheme, app: &App) {
    let area = frame.area();
    let popup_area = centered_rect(40, 3, area);

    frame.render_widget(Clear, popup_area);

    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let ch = spinner_chars[app.spinner.frame % spinner_chars.len()];
    let text = format!("{} {}", ch, app.spinner.message);

    let widget = Paragraph::new(text)
        .style(theme.text_primary_style())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.info)),
        );

    frame.render_widget(widget, popup_area);
}

fn render_popup(frame: &mut Frame, theme: &AkashTheme, app: &App) {
    if let Some(popup) = &app.popup {
        let area = frame.area();
        let popup_area = centered_rect(60, 50, area);

        frame.render_widget(Clear, popup_area);

        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(&popup.content, theme.text_primary_style())),
            Line::from(""),
        ];

        match popup.popup_type {
            crate::tui::widgets::PopupType::Mnemonic => {
                if let Some(ref mnemonic) = app.wallet_state.mnemonic_display {
                    lines.push(Line::from(""));
                    for (i, chunk) in mnemonic
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .chunks(4)
                        .enumerate()
                    {
                        let numbered: Vec<String> = chunk
                            .iter()
                            .enumerate()
                            .map(|(j, w)| format!("{:2}. {}", i * 4 + j + 1, w))
                            .collect();
                        lines.push(Line::from(Span::styled(
                            numbered.join("  "),
                            Style::default().fg(theme.warning),
                        )));
                    }
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(Span::styled(
                    "Press any key to dismiss",
                    theme.text_dim_style(),
                )));
            }
            crate::tui::widgets::PopupType::DeployConfirm => {
                // Show cost breakdown details
                for detail in &popup.details {
                    if detail.is_empty() {
                        lines.push(Line::from(""));
                    } else if detail.starts_with("Press") {
                        lines.push(Line::from(Span::styled(detail.as_str(), theme.text_dim_style())));
                    } else {
                        lines.push(Line::from(Span::styled(detail.as_str(), theme.text_primary_style())));
                    }
                }
            }
            crate::tui::widgets::PopupType::FeeGrantNeeded => {
                // Show fee grant needed details with warning styling
                for detail in &popup.details {
                    if detail.is_empty() {
                        lines.push(Line::from(""));
                    } else if detail.starts_with("Press") {
                        lines.push(Line::from(Span::styled(detail.as_str(), theme.text_dim_style())));
                    } else if detail.contains("Current") || detail.contains("Minimum") {
                        lines.push(Line::from(Span::styled(detail.as_str(), Style::default().fg(theme.warning))));
                    } else {
                        lines.push(Line::from(Span::styled(detail.as_str(), theme.text_primary_style())));
                    }
                }
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    "Press any key to dismiss",
                    theme.text_dim_style(),
                )));
            }
        }

        let border_color = match popup.popup_type {
            crate::tui::widgets::PopupType::FeeGrantNeeded => theme.warning,
            crate::tui::widgets::PopupType::Error => theme.error,
            _ => theme.primary,
        };

        let widget = Paragraph::new(lines).alignment(Alignment::Center).block(
            Block::default()
                .title(Span::styled(&popup.title, Style::default().fg(border_color).bold()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

        frame.render_widget(widget, popup_area);
    }
}

/// Helper to create a centered rect of given percentage width/height
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
