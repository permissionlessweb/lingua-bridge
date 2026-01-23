use ratatui::prelude::*;
use crate::tui::app::App;
use crate::tui::theme::AkashTheme;

mod splash;
mod wallet;
mod fee_grant;
mod deployment;
mod bids;
mod leases;
mod discord_config;
mod deployments;

pub fn render_splash(frame: &mut Frame, theme: &AkashTheme, area: Rect) {
    splash::render(frame, theme, area);
}

pub fn render_wallet(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    wallet::render(frame, theme, app, area);
}

pub fn render_fee_grant(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    fee_grant::render(frame, theme, app, area);
}

pub fn render_deployment(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    deployment::render(frame, theme, app, area);
}

pub fn render_bids(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    bids::render(frame, theme, app, area);
}

pub fn render_leases(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    leases::render(frame, theme, app, area);
}

pub fn render_discord_config(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    discord_config::render(frame, theme, app, area);
}

pub fn render_deployments(frame: &mut Frame, theme: &AkashTheme, app: &App, area: Rect) {
    deployments::render(frame, theme, app, area);
}
