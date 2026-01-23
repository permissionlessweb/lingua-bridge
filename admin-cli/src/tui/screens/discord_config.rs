use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::App;
use crate::tui::input::InputMode;
use crate::tui::theme::AkashTheme;

const GUIDE_STEPS: &[(&str, &str)] = &[
    ("Create Discord Application",
     "Go to discord.com/developers/applications\nClick 'New Application', give it a name."),
    ("Create Bot User",
     "In your app settings, go to 'Bot' tab.\nClick 'Add Bot'. Copy the Bot Token."),
    ("Set Bot Permissions",
     "Under OAuth2 > URL Generator:\nSelect 'bot' scope, then permissions:\nSend Messages, Read Messages, Connect, Speak"),
    ("Invite Bot to Server",
     "Copy the generated OAuth2 URL.\nOpen it in browser, select your server,\nauthorize the bot."),
    ("Enter Tokens",
     "Press 'i' to enter INSERT mode.\n- Bot Token: from Discord developer portal\n- HF Token: from huggingface.co/settings/tokens"),
    ("Set Bot URL",
     "Enter the service URI from your active lease.\nPress 'u' to auto-populate from lease.\nOr enter manually from Leases screen."),
    ("Test & Submit",
     "Press 't' to test the endpoint health.\nPress Enter to save configuration.\nThe bot will connect via the provision API."),
];

pub fn render(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // Title
    let mode_hint = match app.input_mode {
        InputMode::Insert => " [INSERT - Tab: next, Enter: submit, Esc: cancel]",
        _ => " [i: edit, j/k: field, x: clear, u: URL, t: test]",
    };
    let title = Paragraph::new(format!("Discord Bot Setup{}", mode_hint))
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));
    frame.render_widget(title, layout[0]);

    // Content: left = guide + form, right = reference
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(layout[1]);

    // Left panel: step-by-step guide + form
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(10), // 3 fields * 2 lines + borders + padding
        ])
        .split(content_layout[0]);

    // Guide step display
    let step = app.discord_state.guide_step.min(GUIDE_STEPS.len() - 1);
    let (step_title, step_body) = GUIDE_STEPS[step];

    let mut guide_lines = vec![
        Line::from(Span::styled(
            format!("Step {}/{}: {}", step + 1, GUIDE_STEPS.len(), step_title),
            theme.primary_style().bold(),
        )),
        Line::from(""),
    ];

    for line in step_body.lines() {
        guide_lines.push(Line::from(Span::styled(line, theme.text_primary_style())));
    }

    guide_lines.push(Line::from(""));
    guide_lines.push(Line::from(Span::styled(
        "n: next step | p: previous step",
        theme.text_dim_style(),
    )));

    // Progress bar
    guide_lines.push(Line::from(""));
    let progress: String = (0..GUIDE_STEPS.len())
        .map(|i| if i <= step { '#' } else { '-' })
        .collect();
    guide_lines.push(Line::from(vec![
        Span::styled("[", theme.text_dim_style()),
        Span::styled(&progress, Style::default().fg(theme.success)),
        Span::styled("]", theme.text_dim_style()),
    ]));

    let guide_panel = Paragraph::new(guide_lines)
        .block(
            Block::default()
                .title(Span::styled(" Setup Guide ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(guide_panel, left_layout[0]);

    // Form (below guide)
    app.discord_state.form.render(frame, theme, left_layout[1], " Configuration ");

    // Right panel: full reference + status
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(7),
        ])
        .split(content_layout[1]);

    // Full sequence reference
    let mut ref_lines = vec![
        Line::from(Span::styled("Complete Setup Sequence", theme.text_primary_style().bold())),
        Line::from(""),
    ];

    for (i, (title, _)) in GUIDE_STEPS.iter().enumerate() {
        let marker = if i < step {
            Span::styled("[x] ", Style::default().fg(theme.success))
        } else if i == step {
            Span::styled("[>] ", Style::default().fg(theme.primary))
        } else {
            Span::styled("[ ] ", theme.text_dim_style())
        };
        let style = if i == step {
            theme.primary_style()
        } else if i < step {
            theme.text_dim_style()
        } else {
            theme.text_primary_style()
        };
        ref_lines.push(Line::from(vec![
            marker,
            Span::styled(format!("{}. {}", i + 1, title), style),
        ]));
    }

    // Field completion indicators
    ref_lines.push(Line::from(""));
    ref_lines.push(Line::from(Span::styled("Form Status", theme.text_primary_style().bold())));
    ref_lines.push(Line::from(""));
    for field in &app.discord_state.form.fields {
        let (marker, style) = if !field.value.is_empty() {
            ("[x]", Style::default().fg(theme.success))
        } else {
            ("[ ]", theme.text_dim_style())
        };
        ref_lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), style),
            Span::styled(&field.label, style),
        ]));
    }

    ref_lines.push(Line::from(""));
    ref_lines.push(Line::from(Span::styled("Links", theme.text_primary_style().bold())));
    ref_lines.push(Line::from(Span::styled(
        "  discord.com/developers/applications",
        Style::default().fg(theme.info),
    )));
    ref_lines.push(Line::from(Span::styled(
        "  huggingface.co/settings/tokens",
        Style::default().fg(theme.info),
    )));

    let ref_panel = Paragraph::new(ref_lines)
        .block(
            Block::default()
                .title(Span::styled(" Reference ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(ref_panel, right_layout[0]);

    // Status panel
    let status_style = match app.discord_state.deploy_status.as_str() {
        "Not configured" => theme.text_dim_style(),
        s if s.contains("Configured") || s.contains("healthy") => Style::default().fg(theme.success),
        s if s.contains("failed") || s.contains("error") || s.contains("Incomplete") => Style::default().fg(theme.warning),
        _ => Style::default().fg(theme.info),
    };

    let uri_display = app.discord_state.form.get_value("Bot URL");
    let uri_display = if uri_display.is_empty() { "Not set (press 'u')" } else { uri_display };

    let mut status_lines = vec![
        Line::from(vec![
            Span::styled("Status: ", theme.text_dim_style()),
            Span::styled(&*app.discord_state.deploy_status, status_style),
        ]),
        Line::from(vec![
            Span::styled("URI: ", theme.text_dim_style()),
            Span::styled(uri_display, Style::default().fg(theme.info)),
        ]),
    ];

    if app.discord_state.loading {
        status_lines.push(Line::from(Span::styled("Testing...", Style::default().fg(theme.info))));
    }

    let status_panel = Paragraph::new(status_lines)
        .block(
            Block::default()
                .title(Span::styled(" Status ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(status_panel, right_layout[1]);
}
