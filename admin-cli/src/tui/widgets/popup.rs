use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::theme::AkashTheme;

pub enum PopupType {
    Confirm,
    Error,
    Info,
    Mnemonic,
    DeployConfirm,   // Deployment confirmation with cost breakdown
    FeeGrantNeeded,  // Balance too low, suggest fee grant
}

pub struct Popup {
    pub popup_type: PopupType,
    pub title: String,
    pub content: String,
    pub details: Vec<String>,    // Extra lines (cost breakdown, etc.)
    pub buttons: Vec<String>,
    pub visible: bool,
    pub selected_button: usize,
}

impl Popup {
    pub fn new(popup_type: PopupType, title: String, content: String) -> Self {
        Self {
            popup_type,
            title,
            content,
            details: Vec::new(),
            buttons: vec!["OK".to_string()],
            visible: false,
            selected_button: 0,
        }
    }

    pub fn with_details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }

    pub fn with_buttons(mut self, buttons: Vec<String>) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn render(&self, frame: &mut Frame, theme: &AkashTheme, area: Rect) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .title(&*self.title)
            .borders(Borders::ALL)
            .border_style(theme.primary_style());

        frame.render_widget(block, area);
    }
}
