pub mod admin;
pub mod bot;
pub mod config;
pub mod db;
pub mod error;
pub mod translation;
pub mod voice;
pub mod web;

pub use config::AppConfig;
pub use error::{AppError, AppResult};

