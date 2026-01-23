use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::theme::AkashTheme;

pub struct LogViewer {
    pub lines: Vec<String>,
    pub scroll_offset: usize,
    pub max_lines: usize,
}

impl LogViewer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            scroll_offset: 0,
            max_lines,
        }
    }

    pub fn add_line(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            self.lines.remove(0);
        }
        // Auto-scroll to bottom
        let visible = 20usize; // approximate
        if self.lines.len() > visible {
            self.scroll_offset = self.lines.len() - visible;
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.lines.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    pub fn render(&self, frame: &mut Frame, theme: &AkashTheme, area: Rect) {
        let visible_lines = area.height.saturating_sub(2) as usize; // account for borders
        let lines: Vec<Line> = self.lines.iter()
            .skip(self.scroll_offset)
            .take(visible_lines)
            .map(|line| Line::from(line.as_str()).style(theme.text_primary_style()))
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Logs")
                    .borders(Borders::ALL)
                    .border_style(theme.primary_style()),
            );

        frame.render_widget(paragraph, area);
    }
}
