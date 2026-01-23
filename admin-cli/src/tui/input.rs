/// Input modes for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Normal
    }
}
