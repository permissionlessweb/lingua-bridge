pub mod mylang;
pub mod setup;
pub mod translate;
pub mod voice;
pub mod webview;

pub use mylang::{mylang, mypreferences};
pub use setup::setup;
pub use translate::{languages, translate};
pub use voice::{voice, voiceconfig};
pub use webview::webview;

use crate::bot::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Get all registered commands
pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        setup(),
        translate(),
        languages(),
        mylang(),
        mypreferences(),
        webview(),
        voice(),
        voiceconfig(),
    ]
}
