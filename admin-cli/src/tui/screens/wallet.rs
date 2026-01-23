use crate::tui::app::App;
use crate::tui::theme::AkashTheme;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Title
    let title = Paragraph::new("Wallet Setup")
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );

    frame.render_widget(title, layout[0]);

    // Content area
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);

    // Left panel - Actions
    let mut left_lines = vec![
        Line::from(Span::styled("Generate", theme.text_primary_style().bold())),
        Line::from(Span::styled(
            "  g  Generate 24-word mnemonic",
            theme.text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled("Import", theme.text_primary_style().bold())),
        Line::from(Span::styled(
            "  i  Import existing mnemonic (12/24 words)",
            theme.text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled("Clipboard", theme.text_primary_style().bold())),
        Line::from(Span::styled(
            "  c  Copy mnemonic to clipboard",
            theme.text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Persistence",
            theme.text_primary_style().bold(),
        )),
        Line::from(Span::styled(
            "  s  Save wallet (encrypted)",
            theme.text_primary_style(),
        )),
        Line::from(Span::styled(
            "  l  Load saved wallet",
            theme.text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled("Query", theme.text_primary_style().bold())),
        Line::from(Span::styled(
            "  r  Refresh on-chain balance",
            theme.text_primary_style(),
        )),
    ];

    if app.wallet_state.loading {
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "  Loading...",
            Style::default().fg(theme.info),
        )));
    }

    let left_panel = Paragraph::new(left_lines).block(
        Block::default()
            .title(Span::styled(" Actions ", theme.primary_style()))
            .borders(Borders::ALL)
            .border_style(theme.primary_style()),
    );

    frame.render_widget(left_panel, content_layout[0]);

    // Right panel - Current wallet info or import input
    let mut right_lines = vec![];

    if app.wallet_state.importing_mnemonic {
        // Import mode - show input field
        right_lines.push(Line::from(Span::styled(
            "Import Mnemonic",
            theme.text_primary_style().bold(),
        )));
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled(
            "Paste your 12 or 24-word mnemonic:",
            theme.text_primary_style(),
        )));
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled(
            if app.wallet_state.import_text.is_empty() {
                "Enter mnemonic here...".to_string()
            } else {
                app.wallet_state.import_text.clone()
            },
            if app.input_mode == crate::tui::input::InputMode::Insert {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                theme.text_primary_style()
            },
        )));
        right_lines.push(Line::from(""));
        right_lines.push(Line::from(Span::styled(
            "Press Enter to import, Esc to cancel",
            theme.text_dim_style(),
        )));
    } else {
        // Normal mode - show wallet info
        let address_display = app
            .wallet_state
            .wallet
            .address
            .as_deref()
            .unwrap_or("Not loaded");
        let balance_display = app.wallet_state.balance.as_deref().unwrap_or("N/A");

        let status_style = if app.wallet_state.wallet.is_loaded() {
            Style::default().fg(theme.success)
        } else {
            theme.text_dim_style()
        };

        let status_text = if app.wallet_state.wallet.is_loaded() {
            "Loaded"
        } else {
            "No wallet"
        };

        let saved_style = if app.wallet_state.is_saved {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.warning)
        };

        let saved_text = if app.wallet_state.is_saved {
            "Encrypted on disk"
        } else {
            "Not saved"
        };

        right_lines.extend(vec![
            Line::from(Span::styled("Status", theme.text_primary_style().bold())),
            Line::from(""),
            Line::from(Span::styled(status_text, status_style)),
            Line::from(""),
            Line::from(Span::styled("Address", theme.text_primary_style().bold())),
            Line::from(""),
            Line::from(Span::styled(address_display, theme.text_primary_style())),
            Line::from(""),
            Line::from(Span::styled("Balance", theme.text_primary_style().bold())),
            Line::from(""),
            Line::from(Span::styled(balance_display, theme.text_primary_style())),
            Line::from(""),
            Line::from(Span::styled("Storage", theme.text_primary_style().bold())),
            Line::from(""),
            Line::from(Span::styled(saved_text, saved_style)),
        ]);

        if let Some(ref path) = app.wallet_state.encrypted_path {
            right_lines.push(Line::from(Span::styled(
                format!("  {}", path),
                theme.text_dim_style(),
            )));
        }
    }

    let right_panel = Paragraph::new(right_lines).block(
        Block::default()
            .title(Span::styled(" Wallet Info ", theme.primary_style()))
            .borders(Borders::ALL)
            .border_style(theme.primary_style()),
    );

    frame.render_widget(right_panel, content_layout[1]);
}
