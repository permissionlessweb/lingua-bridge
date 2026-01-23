use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::App;
use crate::tui::theme::AkashTheme;

/// Minimum balance threshold for display
const MIN_DEPLOY_BALANCE_UAKT: u64 = 5_000_000;

pub fn render(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),  // Status banner
            Constraint::Min(1),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Fee Grant — Gas Fee Management")
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));
    frame.render_widget(title, layout[0]);

    // Status banner (conditional)
    let balance_uakt = app.fee_grant_state.balance_uakt;
    let has_grant = app.fee_grant_state.has_fee_grant;
    let (banner_text, banner_style) = if balance_uakt >= MIN_DEPLOY_BALANCE_UAKT {
        ("✓ Sufficient balance for deployment — fee grant optional", Style::default().fg(theme.success))
    } else if has_grant {
        ("✓ Fee grant active — deployment fees covered by granter", Style::default().fg(theme.success))
    } else if balance_uakt > 0 {
        ("⚠ Low balance and no fee grant — request one before deploying", Style::default().fg(theme.warning))
    } else if app.wallet_state.wallet.address.is_some() {
        ("⚠ Zero balance — request a fee grant to cover deployment gas fees", Style::default().fg(theme.warning))
    } else {
        ("○ Load or generate a wallet first (Tab 3: Wallet)", Style::default().fg(theme.info))
    };

    let banner = Paragraph::new(banner_text)
        .style(banner_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(
            if balance_uakt >= MIN_DEPLOY_BALANCE_UAKT || has_grant {
                Style::default().fg(theme.success)
            } else {
                Style::default().fg(theme.warning)
            }
        ));
    frame.render_widget(banner, layout[1]);

    // Content area
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(layout[2]);

    // Left panel - Fee grant actions
    let mut left_lines = vec![
        Line::from(Span::styled("Actions", theme.text_primary_style().bold())),
        Line::from(""),
        Line::from(Span::styled("  r  Request fee grant from faucet", theme.text_primary_style())),
        Line::from(Span::styled("     Granter pays gas for your deployments", theme.text_dim_style())),
        Line::from(""),
        Line::from(Span::styled("  c  Check fee grant allowance", theme.text_primary_style())),
        Line::from(Span::styled("     Query active grants for your address", theme.text_dim_style())),
        Line::from(""),
        Line::from(Span::styled("  b  Refresh wallet balance", theme.text_primary_style())),
        Line::from(Span::styled("     Check current uAKT balance on-chain", theme.text_dim_style())),
    ];

    if app.fee_grant_state.loading {
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled("  Loading...", Style::default().fg(theme.info))));
    }

    left_lines.push(Line::from(""));
    left_lines.push(Line::from(Span::styled("Navigation", theme.text_primary_style().bold())));
    left_lines.push(Line::from(""));
    left_lines.push(Line::from(Span::styled("  Tab      Next step (Submit)", theme.text_dim_style())));
    left_lines.push(Line::from(Span::styled("  BackTab  Previous step (SDL)", theme.text_dim_style())));

    let left_panel = Paragraph::new(left_lines)
        .block(
            Block::default()
                .title(Span::styled(" Commands ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(left_panel, content_layout[0]);

    // Right panel - Fee grant info
    let fee_status_style = match app.fee_grant_state.fee_grant_status.as_str() {
        "Not checked" => theme.text_dim_style(),
        s if s.contains("Needed") => Style::default().fg(theme.warning),
        s if s.contains("pending") => Style::default().fg(theme.info),
        s if s.contains("Active") || s.contains("Not needed") => Style::default().fg(theme.success),
        _ => theme.text_primary_style(),
    };

    let balance_display = if balance_uakt > 0 {
        format!("{} uakt ({:.4} AKT)", balance_uakt, balance_uakt as f64 / 1_000_000.0)
    } else {
        app.fee_grant_state.balance.as_deref().unwrap_or("Not checked").to_string()
    };

    let balance_style = if balance_uakt >= MIN_DEPLOY_BALANCE_UAKT {
        Style::default().fg(theme.success)
    } else if balance_uakt > 0 {
        Style::default().fg(theme.warning)
    } else {
        theme.text_dim_style()
    };

    let mut right_lines = vec![
        Line::from(Span::styled("Fee Grant Status", theme.text_primary_style().bold())),
        Line::from(""),
        Line::from(Span::styled(&*app.fee_grant_state.fee_grant_status, fee_status_style)),
        Line::from(""),
        Line::from(Span::styled("Wallet Balance", theme.text_primary_style().bold())),
        Line::from(""),
        Line::from(Span::styled(balance_display, balance_style)),
        Line::from(""),
    ];

    // Show allowance details if available
    if !app.fee_grant_state.allowances.is_empty() {
        right_lines.push(Line::from(Span::styled("Active Grants", theme.text_primary_style().bold())));
        right_lines.push(Line::from(""));
        for grant in &app.fee_grant_state.allowances {
            let granter_short = if grant.granter.len() > 20 {
                format!("{}...{}", &grant.granter[..10], &grant.granter[grant.granter.len()-6..])
            } else {
                grant.granter.clone()
            };
            right_lines.push(Line::from(vec![
                Span::styled("  From: ", theme.text_dim_style()),
                Span::styled(granter_short, Style::default().fg(theme.success)),
            ]));
            if let Some(ref limit) = grant.spend_limit {
                right_lines.push(Line::from(vec![
                    Span::styled("  Limit: ", theme.text_dim_style()),
                    Span::styled(format!("{} {}", limit.amount, limit.denom), theme.text_primary_style()),
                ]));
            } else {
                right_lines.push(Line::from(vec![
                    Span::styled("  Limit: ", theme.text_dim_style()),
                    Span::styled("unlimited", Style::default().fg(theme.success)),
                ]));
            }
            if let Some(ref exp) = grant.expiration {
                right_lines.push(Line::from(vec![
                    Span::styled("  Expires: ", theme.text_dim_style()),
                    Span::styled(exp.as_str(), theme.text_dim_style()),
                ]));
            }
            right_lines.push(Line::from(""));
        }
    } else {
        right_lines.push(Line::from(Span::styled("Allowance", theme.text_primary_style().bold())));
        right_lines.push(Line::from(""));
        let allowance_text = app.fee_grant_state.allowance.as_deref().unwrap_or("None");
        right_lines.push(Line::from(Span::styled(allowance_text, theme.text_dim_style())));
    }

    // Deployment readiness hint
    right_lines.push(Line::from(""));
    right_lines.push(Line::from(Span::styled("Deploy Readiness", theme.text_primary_style().bold())));
    right_lines.push(Line::from(""));
    if balance_uakt >= MIN_DEPLOY_BALANCE_UAKT || has_grant {
        right_lines.push(Line::from(Span::styled("  ✓ Ready to deploy", Style::default().fg(theme.success))));
    } else {
        right_lines.push(Line::from(Span::styled("  ✗ Need fee grant or balance", Style::default().fg(theme.error))));
    }

    let right_panel = Paragraph::new(right_lines)
        .block(
            Block::default()
                .title(Span::styled(" Status ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(right_panel, content_layout[1]);
}
