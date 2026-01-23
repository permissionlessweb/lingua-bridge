use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::theme::AkashTheme;

pub struct FormField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub is_active: bool,
}

pub struct Form {
    pub fields: Vec<FormField>,
    pub active_index: usize,
}

impl Form {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            active_index: 0,
        }
    }

    pub fn add_field(&mut self, label: &str, placeholder: &str) {
        self.fields.push(FormField {
            label: label.to_string(),
            value: String::new(),
            placeholder: placeholder.to_string(),
            is_active: self.fields.is_empty(), // first field starts active
        });
    }

    pub fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.fields[self.active_index].is_active = false;
            self.active_index = (self.active_index + 1) % self.fields.len();
            self.fields[self.active_index].is_active = true;
        }
    }

    pub fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            self.fields[self.active_index].is_active = false;
            self.active_index = if self.active_index == 0 {
                self.fields.len() - 1
            } else {
                self.active_index - 1
            };
            self.fields[self.active_index].is_active = true;
        }
    }

    pub fn input_char(&mut self, c: char) {
        if let Some(field) = self.fields.get_mut(self.active_index) {
            field.value.push(c);
        }
    }

    pub fn delete_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.active_index) {
            field.value.pop();
        }
    }

    pub fn active_value(&self) -> &str {
        self.fields.get(self.active_index)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn get_value(&self, label: &str) -> &str {
        self.fields.iter()
            .find(|f| f.label == label)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn values(&self) -> Vec<(&str, &str)> {
        self.fields.iter().map(|f| (f.label.as_str(), f.value.as_str())).collect()
    }

    pub fn is_complete(&self) -> bool {
        self.fields.iter().all(|f| !f.value.is_empty())
    }

    pub fn clear_active(&mut self) {
        if let Some(field) = self.fields.get_mut(self.active_index) {
            field.value.clear();
        }
    }

    pub fn clear(&mut self) {
        for field in &mut self.fields {
            field.value.clear();
        }
        self.active_index = 0;
        if let Some(f) = self.fields.first_mut() {
            f.is_active = true;
        }
    }

    pub fn render(&self, frame: &mut Frame, theme: &AkashTheme, area: Rect, title: &str) {
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(theme.primary_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let field_height = 2u16; // label + input line
        let constraints: Vec<Constraint> = self.fields.iter()
            .map(|_| Constraint::Length(field_height))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let field_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, field) in self.fields.iter().enumerate() {
            if i >= field_areas.len() - 1 {
                break;
            }
            let field_area = field_areas[i];

            let display_value = if field.value.is_empty() {
                field.placeholder.as_str()
            } else {
                field.value.as_str()
            };

            let style = if field.is_active {
                theme.primary_style()
            } else if field.value.is_empty() {
                theme.text_dim_style()
            } else {
                theme.text_primary_style()
            };

            let label_style = if field.is_active {
                theme.primary_style().bold()
            } else {
                theme.text_primary_style()
            };

            let text = vec![
                Line::from(Span::styled(&field.label, label_style)),
                Line::from(Span::styled(
                    if field.is_active {
                        format!("▸ {}", display_value)
                    } else {
                        format!("  {}", display_value)
                    },
                    style,
                )),
            ];

            let paragraph = Paragraph::new(text);
            frame.render_widget(paragraph, field_area);
        }
    }
}
