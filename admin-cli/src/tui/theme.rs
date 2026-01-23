use ratatui::style::{Color, Style};

/// Akash-themed color palette
pub struct AkashTheme {
    pub primary: Color,
    pub background: Color,
    pub surface: Color,
    pub text_primary: Color,
    pub text_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub mode_normal: Color,
    pub mode_insert: Color,
    pub mode_command: Color,
}

impl Default for AkashTheme {
    fn default() -> Self {
        Self {
            primary: Color::Rgb(229, 62, 62), // #E53E3E - Akash red
            background: Color::Rgb(17, 17, 17), // #111111 - near-black
            surface: Color::Rgb(26, 26, 26),   // #1A1A1A - panel background
            text_primary: Color::Rgb(224, 224, 224), // #E0E0E0
            text_dim: Color::Rgb(128, 128, 128),     // #808080
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            mode_normal: Color::Blue,
            mode_insert: Color::Green,
            mode_command: Color::Yellow,
        }
    }
}

impl AkashTheme {
    /// Get the primary style for titles and active elements
    pub fn primary_style(&self) -> Style {
        Style::default().fg(self.primary)
    }

    /// Get the background style
    pub fn background_style(&self) -> Style {
        Style::default().bg(self.background)
    }

    /// Get the surface style for panels
    pub fn surface_style(&self) -> Style {
        Style::default().bg(self.surface)
    }

    /// Get the primary text style
    pub fn text_primary_style(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Get the dim text style
    pub fn text_dim_style(&self) -> Style {
        Style::default().fg(self.text_dim)
    }

    /// Get style for a specific mode indicator
    pub fn mode_style(&self, mode: crate::tui::input::InputMode) -> Style {
        let color = match mode {
            crate::tui::input::InputMode::Normal => self.mode_normal,
            crate::tui::input::InputMode::Insert => self.mode_insert,
            crate::tui::input::InputMode::Command => self.mode_command,
        };
        Style::default().fg(color)
    }
}
