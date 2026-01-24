use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::{App, DeployPanel};
use crate::tui::input::InputMode;
use crate::tui::theme::AkashTheme;
use crate::tui::ui::centered_rect;

pub fn render(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // Title with mode hint
    let panel_name = match app.deployment_state.active_panel {
        DeployPanel::Variables => "Variables",
        DeployPanel::Services => "Services",
    };
    let mode_hint = match app.input_mode {
        InputMode::Insert => " [INSERT - Enter: confirm, Esc: cancel]",
        _ => " [v: panel, i: edit, j/k: nav, d: deploy]",
    };
    let title = Paragraph::new(format!("SDL Config [{}]{}", panel_name, mode_hint))
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));
    frame.render_widget(title, layout[0]);

    // Check for SDL errors
    if let Some(ref err) = app.deployment_state.sdl_error {
        let error_msg = Paragraph::new(format!("SDL Error: {}", err))
            .style(Style::default().fg(theme.error))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Error ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.error)),
            );
        frame.render_widget(error_msg, layout[1]);
        return;
    }

    let sdl = match &app.deployment_state.sdl {
        Some(s) => s,
        None => return,
    };

    // Content: left = YAML view, right = variables + services
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(layout[1]);

    // Left panel: YAML preview with variable highlighting
    let yaml_lines: Vec<Line> = sdl.raw.lines()
        .skip(app.deployment_state.yaml_scroll)
        .map(|line| {
            // Highlight lines containing template variables
            let has_var = line.contains('<') && line.contains('>');
            let style = if line.trim().starts_with('#') {
                theme.text_dim_style()
            } else if has_var {
                Style::default().fg(theme.warning)
            } else {
                theme.text_primary_style()
            };
            Line::from(Span::styled(line, style))
        })
        .collect();

    let yaml_panel = Paragraph::new(yaml_lines)
        .block(
            Block::default()
                .title(Span::styled(" deploy.yaml ", theme.primary_style()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(yaml_panel, content_layout[0]);

    // Right panel: split into variables (top) and services (bottom)
    let has_vars = !sdl.variables.is_empty();
    let var_height = if has_vars {
        (sdl.variables.len() as u16 + 3).min(12)
    } else {
        0
    };

    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(var_height),   // Variables
            Constraint::Min(1),              // Services + fields
            Constraint::Length(3),           // Status
        ])
        .split(content_layout[1]);

    // Variables panel
    if has_vars {
        let is_var_panel = app.deployment_state.active_panel == DeployPanel::Variables;
        let is_insert = app.input_mode == InputMode::Insert;
        let vi = app.deployment_state.selected_var;

        let mut var_lines: Vec<Line> = Vec::new();
        for (i, var) in sdl.variables.iter().enumerate() {
            let active = is_var_panel && i == vi;
            let display = if active && is_insert {
                format!("{}|", app.deployment_state.editing_value)
            } else if var.value.is_empty() {
                format!("<{}>", var.name)
            } else {
                var.value.clone()
            };

            let prefix = if active { ">" } else { " " };
            let name_style = if active {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_dim_style()
            };
            let val_style = if active {
                Style::default().fg(theme.primary).bold()
            } else if var.value.is_empty() {
                Style::default().fg(theme.warning)
            } else {
                Style::default().fg(theme.success)
            };

            var_lines.push(Line::from(vec![
                Span::styled(format!("{} ", prefix), name_style),
                Span::styled(format!("{}: ", var.name), name_style),
                Span::styled(display, val_style),
            ]));
        }

        let filled = sdl.variables.iter().filter(|v| !v.value.is_empty()).count();
        let total = sdl.variables.len();
        let var_title = format!(" Variables ({}/{}) ", filled, total);

        let border_style = if is_var_panel {
            Style::default().fg(theme.primary)
        } else {
            theme.text_dim_style()
        };

        let var_panel = Paragraph::new(var_lines)
            .block(
                Block::default()
                    .title(Span::styled(var_title, theme.primary_style()))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
        frame.render_widget(var_panel, right_layout[0]);
    }

    // Services panel
    let is_svc_panel = app.deployment_state.active_panel == DeployPanel::Services;
    let svc_border = if is_svc_panel {
        Style::default().fg(theme.primary)
    } else {
        theme.text_dim_style()
    };

    if let Some(svc) = sdl.services.get(app.deployment_state.selected_service) {
        let fi = app.deployment_state.selected_field;
        let is_insert = app.input_mode == InputMode::Insert && is_svc_panel;

        let mut field_lines: Vec<Line> = Vec::new();

        // Service selector
        for (i, s) in sdl.services.iter().enumerate() {
            let marker = if i == app.deployment_state.selected_service { ">" } else { " " };
            let style = if i == app.deployment_state.selected_service {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_dim_style()
            };
            field_lines.push(Line::from(Span::styled(
                format!("{} {}", marker, s.name),
                style,
            )));
        }
        field_lines.push(Line::from(""));

        // Resource fields (indices 0-3)
        let resource_labels = ["CPU", "Memory", "Storage", "GPU"];
        let resource_values = [&svc.resources.cpu, &svc.resources.memory, &svc.resources.storage, &svc.resources.gpu];

        field_lines.push(Line::from(Span::styled("Resources", theme.text_primary_style().bold())));
        for (i, (label, val)) in resource_labels.iter().zip(resource_values.iter()).enumerate() {
            let active = is_svc_panel && fi == i;
            let display = if active && is_insert {
                format!("{}|", app.deployment_state.editing_value)
            } else {
                val.to_string()
            };
            let style = if active {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_primary_style()
            };
            let prefix = if active { ">" } else { " " };
            field_lines.push(Line::from(vec![
                Span::styled(format!("{} {}: ", prefix, label), theme.text_dim_style()),
                Span::styled(display, style),
            ]));
        }

        // Env var fields (indices 4+)
        field_lines.push(Line::from(""));
        field_lines.push(Line::from(Span::styled("Environment", theme.text_primary_style().bold())));
        for (i, env) in svc.env_vars.iter().enumerate() {
            let idx = i + 4;
            let active = is_svc_panel && fi == idx;
            let display = if active && is_insert {
                format!("{}|", app.deployment_state.editing_value)
            } else {
                env.value.clone()
            };
            let style = if active {
                Style::default().fg(theme.primary).bold()
            } else {
                theme.text_primary_style()
            };
            let prefix = if active { ">" } else { " " };
            field_lines.push(Line::from(vec![
                Span::styled(format!("{} {}: ", prefix, env.key), theme.text_dim_style()),
                Span::styled(display, style),
            ]));
        }

        let fields_panel = Paragraph::new(field_lines)
            .block(
                Block::default()
                    .title(Span::styled(
                        format!(" {} ", svc.name),
                        theme.primary_style(),
                    ))
                    .borders(Borders::ALL)
                    .border_style(svc_border),
            );
        frame.render_widget(fields_panel, right_layout[1]);
    }

    // Status bar
    let selected_gpus = app.deployment_state.gpu_catalog.selected_models();
    let gpu_text = if selected_gpus.is_empty() {
        String::new()
    } else {
        format!(" | GPU: {}", selected_gpus.join(", "))
    };
    let status_style = match app.deployment_state.status.as_str() {
        "Not deployed" => theme.text_dim_style(),
        "Submitting..." => Style::default().fg(theme.info),
        _ => Style::default().fg(theme.success),
    };
    let dseq_text = app.deployment_state.dseq
        .map(|d| format!(" | DSeq: {}", d))
        .unwrap_or_default();
    let status_line = Paragraph::new(format!("{}{}{}", app.deployment_state.status, dseq_text, gpu_text))
        .style(status_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(theme.primary_style()));
    frame.render_widget(status_line, right_layout[2]);

    // GPU picker overlay
    if app.deployment_state.gpu_picker_open {
        render_gpu_picker(frame, theme, app, area);
    }
}

fn render_gpu_picker(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    // Centered overlay
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let catalog = &app.deployment_state.gpu_catalog;
    let selected_idx = app.deployment_state.gpu_selected_index;

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Select GPUs (Space: toggle, g/Esc: close)",
        theme.text_dim_style(),
    )));
    lines.push(Line::from(""));

    for (i, model) in catalog.unique_models.iter().enumerate() {
        let is_current = i == selected_idx;
        let checkbox = if model.selected { "[x]" } else { "[ ]" };
        let marker = if is_current { ">" } else { " " };

        // Best variant info
        let best_mem = model.variants.iter()
            .map(|v| v.memory_size.as_str())
            .max()
            .unwrap_or("?");

        let style = if is_current {
            Style::default().fg(theme.primary).bold()
        } else if model.selected {
            Style::default().fg(theme.success)
        } else {
            theme.text_primary_style()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} {} ", marker, checkbox), style),
            Span::styled(&model.name, style),
            Span::styled(format!("  ({})", best_mem), theme.text_dim_style()),
        ]));
    }

    let selected_count = catalog.unique_models.iter().filter(|m| m.selected).count();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("{} GPU model(s) selected", selected_count),
        Style::default().fg(theme.info),
    )));

    let picker = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" GPU Models ", theme.primary_style().bold()))
                .borders(Borders::ALL)
                .border_style(theme.primary_style()),
        );
    frame.render_widget(picker, popup_area);
}
