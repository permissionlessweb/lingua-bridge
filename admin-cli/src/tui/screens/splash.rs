use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::theme::AkashTheme;

pub fn render(frame: &mut Frame, theme: &AkashTheme, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    // ASCII Art placeholder for Akash logo
    let logo = r#"
    █████╗ ██╗  ██╗ █████╗ ███████╗██╗  ██╗
    ██╔══██╗██║ ██╔╝██╔══██╗██╔════╝██║  ██║
    ███████║█████╔╝ ███████║███████╗███████║
    ██╔══██║██╔═██╗ ██╔══██║╚════██║██╔══██║
    ██║  ██║██║  ██╗██║  ██║███████║██║  ██║
    ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
    "#;

    let logo_widget = Paragraph::new(logo)
        .style(theme.primary_style().bold())
        .alignment(Alignment::Center)
        .block(Block::default());

    frame.render_widget(logo_widget, layout[0]);

    // Title and subtitle
    let title = Paragraph::new("LinguaBridge Admin")
        .style(theme.text_primary_style().bold())
        .alignment(Alignment::Center);

    let subtitle = Paragraph::new("Press any key to continue...")
        .style(theme.text_dim_style())
        .alignment(Alignment::Center);

    frame.render_widget(title, layout[1]);
    frame.render_widget(subtitle, layout[2]);
}
