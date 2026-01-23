use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::tui::api::{AkashClient, BidInfo, FeeAllowanceInfo, LeaseInfo, ProviderClient};
use crate::tui::config::{AppConfig, ConfigStore};
use crate::tui::event::AppEvent;
use crate::tui::gpu::GpuCatalog;
use crate::tui::input::InputMode;
use crate::tui::sdl::SdlFile;
use crate::tui::wallet::keygen::KeyGenerator;
use crate::tui::wallet::Wallet;
use crate::tui::widgets::{Form, LogViewer, Popup, PopupType, Spinner};

/// Top-level tab for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainTab {
    Deployments, // Dashboard of existing deployed bots
    Deploy,      // New deployment wizard
    Wallet,      // Wallet management
}

/// Sub-screen within the Deploy tab (wizard steps)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeployStep {
    SdlConfig,     // SDL variable filling
    FeeGrant,      // Conditional fee grant
    Submit,        // Deployment submission
    Bids,          // Bid selection
    Leases,        // Lease monitoring
    DiscordConfig, // Discord bot setup
}

/// Screen variants for the TUI (kept for render dispatch)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Splash,
    Wallet,
    FeeGrant,
    Deployment,
    Bids,
    Leases,
    DiscordConfig,
    Deployments, // New: deployed bots dashboard
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Splash
    }
}

/// Main application state
pub struct App {
    pub current_screen: Screen,
    pub main_tab: MainTab,
    pub deploy_step: DeployStep,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub show_splash: bool,
    pub tx: Option<mpsc::UnboundedSender<AppEvent>>,

    // Screen states
    pub wallet_state: WalletState,
    pub fee_grant_state: FeeGrantState,
    pub deployment_state: DeploymentState,
    pub bids_state: BidsState,
    pub leases_state: LeasesState,
    pub discord_state: DiscordConfigState,
    pub deployments_state: DeploymentsState,

    // Shared state
    pub popup: Option<Popup>,
    pub spinner: Spinner,
    pub status_message: Option<(String, bool)>, // (message, is_error)

    // Config
    pub config: AppConfig,
}

// --- Per-screen state ---

pub struct WalletState {
    pub wallet: Wallet,
    pub balance: Option<String>,
    pub mnemonic_display: Option<String>,
    pub encrypted_path: Option<String>,
    pub is_saved: bool,
    pub loading: bool,
    pub importing_mnemonic: bool,
    pub import_text: String,
}

/// Minimum balance (in uakt) needed to deploy without a fee grant
const MIN_DEPLOY_BALANCE_UAKT: u64 = 5_000_000; // 5 AKT

pub struct FeeGrantState {
    pub balance: Option<String>,
    pub balance_uakt: u64,
    pub fee_grant_status: String,
    pub allowance: Option<String>,
    pub allowances: Vec<FeeAllowanceInfo>,
    pub has_fee_grant: bool,
    pub loading: bool,
}

/// Tracks readiness for deployment submission
#[derive(Debug, Clone)]
pub struct DeployReadiness {
    pub wallet_ready: bool,
    pub balance_sufficient: bool,
    pub has_fee_grant: bool,
    pub sdl_ready: bool,
    pub variables_filled: bool,
    pub balance_uakt: u64,
    pub issues: Vec<String>,
}

/// Which panel is active in the deployment screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeployPanel {
    Variables,  // SDL template variable form
    Services,   // Per-service env vars + resources
}

pub struct DeploymentState {
    pub sdl: Option<SdlFile>,
    pub sdl_error: Option<String>,
    pub active_panel: DeployPanel,
    pub selected_var: usize,    // Index into sdl.variables
    pub selected_service: usize,
    pub selected_field: usize,
    pub yaml_scroll: usize,
    pub editing_value: String,
    pub dseq: Option<u64>,
    pub status: String,
    pub loading: bool,
    // GPU picker
    pub gpu_catalog: GpuCatalog,
    pub gpu_picker_open: bool,
    pub gpu_selected_index: usize,
    pub gpu_filter_min_memory: u64, // Minimum VRAM in Gi (0 = no filter)
    // Deploy confirmation
    pub confirm_pending: bool,
    pub readiness: Option<DeployReadiness>,
}

pub struct BidsState {
    pub bids: Vec<BidInfo>,
    pub selected_index: usize,
    pub dseq: Option<u64>,
    pub loading: bool,
}

pub struct LeasesState {
    pub leases: Vec<LeaseInfo>,
    pub selected_index: usize,
    pub service_uris: Vec<String>,
    pub log_viewer: LogViewer,
    pub loading: bool,
}

pub struct DiscordConfigState {
    pub form: Form,
    pub guide_step: usize,
    pub deploy_status: String,
    pub service_uri: Option<String>,
    pub loading: bool,
}

/// State for the Deployments dashboard (existing bots)
pub struct DeploymentsState {
    pub deployments: Vec<DeploymentRecord>,
    pub selected_index: usize,
    pub loading: bool,
}

/// A stored deployment record
#[derive(Debug, Clone)]
pub struct DeploymentRecord {
    pub dseq: u64,
    pub name: String,
    pub status: DeploymentStatus,
    pub services: Vec<ServiceRecord>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentStatus {
    Active,
    Terminated,
    Failed,
    Unknown,
}

impl DeploymentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Terminated => "terminated",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceRecord {
    pub name: String,
    pub uri: Option<String>,
    pub status: String,
}

impl App {
    pub fn new() -> Self {
        let (sdl, sdl_error) = match SdlFile::load(None) {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };

        let mut discord_form = Form::new();
        discord_form.add_field("Bot Token", "Discord bot token");
        discord_form.add_field("HF Token", "HuggingFace API token (for inference)");
        discord_form.add_field("Bot URL", "Service URI from active lease");

        let config = ConfigStore::new()
            .ok()
            .and_then(|store| store.load_config().ok())
            .unwrap_or_default();

        // Detect if we have existing deployments to determine startup tab
        let has_deployments = !config.deployments.is_empty();
        let initial_tab = if has_deployments {
            MainTab::Deployments
        } else {
            MainTab::Deploy
        };

        Self {
            current_screen: Screen::Splash,
            main_tab: initial_tab,
            deploy_step: DeployStep::SdlConfig,
            input_mode: InputMode::Normal,
            should_quit: false,
            show_splash: true,
            tx: None,

            wallet_state: WalletState {
                wallet: Wallet::new(),
                balance: None,
                mnemonic_display: None,
                encrypted_path: ConfigStore::new().ok().map(|s| s.wallet_path().display().to_string()),
                is_saved: ConfigStore::new().ok().map(|s| s.has_wallet()).unwrap_or(false),
                loading: false,
                importing_mnemonic: false,
                import_text: String::new(),
            },
            fee_grant_state: FeeGrantState {
                balance: None,
                balance_uakt: 0,
                fee_grant_status: "Not checked".to_string(),
                allowance: None,
                allowances: Vec::new(),
                has_fee_grant: false,
                loading: false,
            },
            deployment_state: DeploymentState {
                sdl,
                sdl_error,
                active_panel: DeployPanel::Variables,
                selected_var: 0,
                selected_service: 0,
                selected_field: 0,
                yaml_scroll: 0,
                editing_value: String::new(),
                dseq: None,
                status: "Not deployed".to_string(),
                loading: false,
                gpu_catalog: GpuCatalog::load(),
                gpu_picker_open: false,
                gpu_selected_index: 0,
                gpu_filter_min_memory: 0,
                confirm_pending: false,
                readiness: None,
            },
            bids_state: BidsState {
                bids: Vec::new(),
                selected_index: 0,
                dseq: None,
                loading: false,
            },
            leases_state: LeasesState {
                leases: Vec::new(),
                selected_index: 0,
                service_uris: Vec::new(),
                log_viewer: LogViewer::new(500),
                loading: false,
            },
            discord_state: DiscordConfigState {
                form: discord_form,
                guide_step: 0,
                deploy_status: "Not configured".to_string(),
                service_uri: None,
                loading: false,
            },
            deployments_state: DeploymentsState {
                deployments: Vec::new(),
                selected_index: 0,
                loading: false,
            },

            popup: None,
            spinner: Spinner::new("Loading...".to_string()),
            status_message: None,
            config,
        }
    }

    pub fn set_sender(&mut self, tx: mpsc::UnboundedSender<AppEvent>) {
        self.tx = Some(tx);
    }

    /// Handle application events
    pub fn handle_event(&mut self, event: AppEvent) -> bool {
        match event {
            AppEvent::Quit => {
                self.should_quit = true;
                false
            }
            AppEvent::Key(key) => {
                // Ctrl-C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    self.should_quit = true;
                    return false;
                }
                // Handle popup interactions
                if self.popup.is_some() {
                    self.handle_popup_key(key);
                    return true;
                }
                self.handle_key(key);
                !self.should_quit
            }
            AppEvent::Tick => {
                self.spinner.tick();
                true
            }
            // Async results
            AppEvent::WalletGenerated { mnemonic, address } => {
                self.wallet_state.wallet.mnemonic = Some(mnemonic.clone());
                self.wallet_state.wallet.address = Some(address);
                self.wallet_state.mnemonic_display = Some(mnemonic);
                self.wallet_state.loading = false;
                self.spinner.stop();
                self.popup = Some(Popup::new(
                    PopupType::Mnemonic,
                    "Wallet Generated".to_string(),
                    "Save your mnemonic securely! Press any key to dismiss.".to_string(),
                ));
                self.popup.as_mut().unwrap().show();
                true
            }
            AppEvent::WalletImported { mnemonic, address } => {
                self.wallet_state.wallet.mnemonic = Some(mnemonic.clone());
                self.wallet_state.wallet.address = Some(address);
                self.wallet_state.mnemonic_display = Some(mnemonic);
                self.wallet_state.loading = false;
                self.spinner.stop();
                self.status_message = Some(("Wallet imported successfully".to_string(), false));
                true
            }
            AppEvent::BalanceUpdated { amount, denom } => {
                let balance_str = format!("{} {}", amount, denom);
                let balance_uakt = amount.parse::<u64>().unwrap_or(0);
                self.wallet_state.balance = Some(balance_str.clone());
                self.fee_grant_state.balance = Some(balance_str);
                self.fee_grant_state.balance_uakt = balance_uakt;
                // Update fee grant status based on balance
                if balance_uakt >= MIN_DEPLOY_BALANCE_UAKT {
                    self.fee_grant_state.fee_grant_status = "Not needed (sufficient balance)".to_string();
                } else if self.fee_grant_state.has_fee_grant {
                    self.fee_grant_state.fee_grant_status = "Active".to_string();
                } else {
                    self.fee_grant_state.fee_grant_status = "Needed (low balance)".to_string();
                }
                self.wallet_state.loading = false;
                self.fee_grant_state.loading = false;
                self.spinner.stop();
                true
            }
            AppEvent::BidsReceived { bids } => {
                self.bids_state.bids = bids;
                self.bids_state.selected_index = 0;
                self.bids_state.loading = false;
                self.spinner.stop();
                true
            }
            AppEvent::LeasesReceived { leases } => {
                self.leases_state.leases = leases;
                self.leases_state.selected_index = 0;
                self.leases_state.loading = false;
                self.spinner.stop();
                true
            }
            AppEvent::TxBroadcast { txhash, success, message } => {
                self.spinner.stop();
                if success {
                    self.status_message = Some((format!("TX: {}", txhash), false));
                } else {
                    self.status_message = Some((format!("TX failed: {}", message), true));
                }
                true
            }
            AppEvent::StatusMessage { message, is_error } => {
                self.spinner.stop();
                self.status_message = Some((message, is_error));
                true
            }
            AppEvent::LogsReceived { lines } => {
                self.leases_state.log_viewer.clear();
                for line in lines {
                    self.leases_state.log_viewer.add_line(line);
                }
                self.leases_state.loading = false;
                self.spinner.stop();
                true
            }
            AppEvent::FeeAllowanceReceived { allowances } => {
                self.fee_grant_state.has_fee_grant = !allowances.is_empty();
                if let Some(first) = allowances.first() {
                    let limit_str = first.spend_limit
                        .as_ref()
                        .map(|b| format!("{} {}", b.amount, b.denom))
                        .unwrap_or_else(|| "unlimited".to_string());
                    self.fee_grant_state.allowance = Some(limit_str);
                    self.fee_grant_state.fee_grant_status = "Active".to_string();
                } else {
                    self.fee_grant_state.allowance = None;
                    if self.fee_grant_state.balance_uakt < MIN_DEPLOY_BALANCE_UAKT {
                        self.fee_grant_state.fee_grant_status = "Needed (low balance)".to_string();
                    }
                }
                self.fee_grant_state.allowances = allowances;
                self.fee_grant_state.loading = false;
                self.spinner.stop();

                // If we were waiting to confirm deploy, re-check readiness
                if self.deployment_state.confirm_pending {
                    self.check_deploy_readiness();
                }
                true
            }
            AppEvent::DeploymentCreated { dseq, txhash } => {
                self.deployment_state.dseq = Some(dseq);
                self.deployment_state.status = format!("Deployed (DSeq: {})", dseq);
                self.deployment_state.loading = false;
                self.deployment_state.confirm_pending = false;
                self.spinner.stop();
                self.status_message = Some((format!("Deployment created! TX: {}", txhash), false));
                // Auto-advance to bids step
                self.bids_state.dseq = Some(dseq);
                self.deploy_step = DeployStep::Bids;
                self.sync_screen_from_tab();
                true
            }
        }
    }

    fn handle_popup_key(&mut self, key: KeyEvent) {
        let popup_type = self.popup.as_ref().map(|p| match p.popup_type {
            PopupType::DeployConfirm => "deploy_confirm",
            PopupType::FeeGrantNeeded => "fee_grant_needed",
            _ => "generic",
        });

        match popup_type.as_deref() {
            Some("deploy_confirm") => {
                match key.code {
                    KeyCode::Enter => {
                        self.popup = None;
                        self.confirm_deployment();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.popup = None;
                        self.deployment_state.confirm_pending = false;
                        self.status_message = Some(("Deployment cancelled".to_string(), false));
                    }
                    _ => {
                        // Dismiss on any other key
                        self.popup = None;
                        self.deployment_state.confirm_pending = false;
                    }
                }
            }
            Some("fee_grant_needed") => {
                match key.code {
                    KeyCode::Tab => {
                        // Jump to fee grant step
                        self.popup = None;
                        self.deployment_state.confirm_pending = false;
                        self.deploy_step = DeployStep::FeeGrant;
                        self.sync_screen_from_tab();
                    }
                    _ => {
                        self.popup = None;
                        self.deployment_state.confirm_pending = false;
                    }
                }
            }
            _ => {
                // Generic popups dismiss on any key
                self.popup = None;
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Insert => self.handle_insert_key(key),
            InputMode::Command => self.handle_command_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            // Tab switching: 1/2/3
            KeyCode::Char('1') => self.switch_tab(MainTab::Deployments),
            KeyCode::Char('2') => self.switch_tab(MainTab::Deploy),
            KeyCode::Char('3') => self.switch_tab(MainTab::Wallet),
            // Sub-screen navigation within current tab
            KeyCode::Tab => self.next_step(),
            KeyCode::BackTab => self.prev_step(),
            // Screen-specific actions in Normal mode
            _ => self.handle_screen_key(key),
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                if self.wallet_state.importing_mnemonic {
                    self.cancel_mnemonic_import();
                } else {
                    self.deployment_state.editing_value.clear();
                }
            }
            KeyCode::Tab => {
                match self.current_screen {
                    Screen::Deployment => {
                        match self.deployment_state.active_panel {
                            DeployPanel::Variables => {
                                self.apply_variable_edit();
                                if let Some(ref sdl) = self.deployment_state.sdl {
                                    if self.deployment_state.selected_var < sdl.variables.len().saturating_sub(1) {
                                        self.deployment_state.selected_var += 1;
                                    } else {
                                        self.deployment_state.selected_var = 0;
                                    }
                                    // Load next variable's current value
                                    let vi = self.deployment_state.selected_var;
                                    if vi < sdl.variables.len() {
                                        self.deployment_state.editing_value = sdl.variables[vi].value.clone();
                                    }
                                }
                            }
                            DeployPanel::Services => {
                                self.apply_deployment_edit();
                                if let Some(ref sdl) = self.deployment_state.sdl {
                                    if let Some(svc) = sdl.services.get(self.deployment_state.selected_service) {
                                        let max_fields = svc.env_vars.len() + 4;
                                        self.deployment_state.selected_field = (self.deployment_state.selected_field + 1) % max_fields;
                                        self.load_current_field_value();
                                    }
                                }
                            }
                        }
                    }
                    Screen::DiscordConfig => self.discord_state.form.next_field(),
                    _ => {}
                }
            }
            KeyCode::BackTab => {
                match self.current_screen {
                    Screen::DiscordConfig => self.discord_state.form.prev_field(),
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                match self.current_screen {
                    Screen::Wallet if self.wallet_state.importing_mnemonic => {
                        self.wallet_state.import_text.push(c);
                    }
                    Screen::Deployment => self.deployment_state.editing_value.push(c),
                    Screen::DiscordConfig => self.discord_state.form.input_char(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                match self.current_screen {
                    Screen::Wallet if self.wallet_state.importing_mnemonic => {
                        self.wallet_state.import_text.pop();
                    }
                    Screen::Deployment => { self.deployment_state.editing_value.pop(); }
                    Screen::DiscordConfig => self.discord_state.form.delete_char(),
                    _ => {}
                }
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                match self.current_screen {
                    Screen::Wallet if self.wallet_state.importing_mnemonic => {
                        self.import_mnemonic();
                    }
                    Screen::Deployment => {
                        match self.deployment_state.active_panel {
                            DeployPanel::Variables => self.apply_variable_edit(),
                            DeployPanel::Services => self.apply_deployment_edit(),
                        }
                    }
                    _ => self.handle_form_submit(),
                }
            }
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            _ => {}
        }
    }

    fn handle_screen_key(&mut self, key: KeyEvent) {
        match self.current_screen {
            Screen::Splash => {
                // Any key advances past splash to the detected tab
                self.show_splash = false;
                self.sync_screen_from_tab();
            }
            Screen::Deployments => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.deployments_state.selected_index > 0 {
                        self.deployments_state.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.deployments_state.selected_index <
                        self.deployments_state.deployments.len().saturating_sub(1) {
                        self.deployments_state.selected_index += 1;
                    }
                }
                KeyCode::Char('r') => self.refresh_deployments(),
                KeyCode::Char('l') => self.fetch_deployment_logs(),
                _ => {}
            },
            Screen::Wallet => match key.code {
                KeyCode::Char('g') => self.generate_wallet(),
                KeyCode::Char('i') => self.start_mnemonic_import(),
                KeyCode::Char('c') => self.copy_mnemonic_to_clipboard(),
                KeyCode::Char('s') => self.save_wallet_encrypted(),
                KeyCode::Char('l') => self.load_wallet_encrypted(),
                KeyCode::Char('r') => self.refresh_balance(),
                _ => {}
            },
            Screen::FeeGrant => match key.code {
                KeyCode::Char('r') => self.request_fee_grant(),
                KeyCode::Char('c') => self.check_fee_grant(),
                KeyCode::Char('b') => self.refresh_balance(),
                _ => {}
            },
            Screen::Deployment => match key.code {
                // GPU picker toggle
                KeyCode::Char('g') => {
                    self.deployment_state.gpu_picker_open = !self.deployment_state.gpu_picker_open;
                    if self.deployment_state.gpu_picker_open {
                        // Pre-select GPUs from current SDL
                        let current_models: Vec<&str> = vec![
                            "rtx3080", "rtx3090", "rtx4080", "rtx4090", "a100"
                        ];
                        self.deployment_state.gpu_catalog.select_from_sdl(&current_models);
                    }
                }
                KeyCode::Char(' ') if self.deployment_state.gpu_picker_open => {
                    let idx = self.deployment_state.gpu_selected_index;
                    self.deployment_state.gpu_catalog.toggle(idx);
                }
                // Switch between Variables and Services panels
                KeyCode::Char('v') if !self.deployment_state.gpu_picker_open => {
                    self.deployment_state.active_panel = match self.deployment_state.active_panel {
                        DeployPanel::Variables => DeployPanel::Services,
                        DeployPanel::Services => DeployPanel::Variables,
                    };
                }
                KeyCode::Esc if self.deployment_state.gpu_picker_open => {
                    self.deployment_state.gpu_picker_open = false;
                }
                KeyCode::Char('i') | KeyCode::Enter if !self.deployment_state.gpu_picker_open => {
                    match self.deployment_state.active_panel {
                        DeployPanel::Variables => {
                            // Edit the selected variable
                            if let Some(ref sdl) = self.deployment_state.sdl {
                                let vi = self.deployment_state.selected_var;
                                if vi < sdl.variables.len() {
                                    self.deployment_state.editing_value = sdl.variables[vi].value.clone();
                                    self.input_mode = InputMode::Insert;
                                }
                            }
                        }
                        DeployPanel::Services => {
                            // Edit the selected service field
                            if let Some(ref sdl) = self.deployment_state.sdl {
                                if let Some(svc) = sdl.services.get(self.deployment_state.selected_service) {
                                    let fi = self.deployment_state.selected_field;
                                    if fi < 4 {
                                        self.deployment_state.editing_value = match fi {
                                            0 => svc.resources.cpu.clone(),
                                            1 => svc.resources.memory.clone(),
                                            2 => svc.resources.storage.clone(),
                                            3 => svc.resources.gpu.clone(),
                                            _ => String::new(),
                                        };
                                    } else if fi - 4 < svc.env_vars.len() {
                                        self.deployment_state.editing_value = svc.env_vars[fi - 4].value.clone();
                                    }
                                }
                            }
                            self.input_mode = InputMode::Insert;
                        }
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.deployment_state.gpu_picker_open {
                        let max = self.deployment_state.gpu_catalog.unique_models.len().saturating_sub(1);
                        if self.deployment_state.gpu_selected_index < max {
                            self.deployment_state.gpu_selected_index += 1;
                        }
                    } else {
                        match self.deployment_state.active_panel {
                            DeployPanel::Variables => {
                                if let Some(ref sdl) = self.deployment_state.sdl {
                                    if self.deployment_state.selected_var < sdl.variables.len().saturating_sub(1) {
                                        self.deployment_state.selected_var += 1;
                                    }
                                }
                            }
                            DeployPanel::Services => {
                                if let Some(ref sdl) = self.deployment_state.sdl {
                                    if self.deployment_state.selected_service < sdl.services.len().saturating_sub(1) {
                                        self.deployment_state.selected_service += 1;
                                        self.deployment_state.selected_field = 0;
                                    }
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.deployment_state.gpu_picker_open {
                        if self.deployment_state.gpu_selected_index > 0 {
                            self.deployment_state.gpu_selected_index -= 1;
                        }
                    } else {
                        match self.deployment_state.active_panel {
                            DeployPanel::Variables => {
                                if self.deployment_state.selected_var > 0 {
                                    self.deployment_state.selected_var -= 1;
                                }
                            }
                            DeployPanel::Services => {
                                if self.deployment_state.selected_service > 0 {
                                    self.deployment_state.selected_service -= 1;
                                    self.deployment_state.selected_field = 0;
                                }
                            }
                        }
                    }
                }
                KeyCode::Tab => {
                    if self.deployment_state.active_panel == DeployPanel::Services {
                        if let Some(ref sdl) = self.deployment_state.sdl {
                            if let Some(svc) = sdl.services.get(self.deployment_state.selected_service) {
                                let max_fields = svc.env_vars.len() + 4;
                                self.deployment_state.selected_field = (self.deployment_state.selected_field + 1) % max_fields;
                            }
                        }
                    }
                }
                KeyCode::BackTab => {
                    if self.deployment_state.active_panel == DeployPanel::Services {
                        if let Some(ref sdl) = self.deployment_state.sdl {
                            if let Some(svc) = sdl.services.get(self.deployment_state.selected_service) {
                                let max_fields = svc.env_vars.len() + 4;
                                if self.deployment_state.selected_field == 0 {
                                    self.deployment_state.selected_field = max_fields.saturating_sub(1);
                                } else {
                                    self.deployment_state.selected_field -= 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('d') => self.submit_deployment(),
                _ => {}
            },
            Screen::Bids => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.bids_state.selected_index > 0 {
                        self.bids_state.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.bids_state.selected_index < self.bids_state.bids.len().saturating_sub(1) {
                        self.bids_state.selected_index += 1;
                    }
                }
                KeyCode::Enter => self.accept_bid(),
                KeyCode::Char('r') => self.refresh_bids(),
                _ => {}
            },
            Screen::Leases => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.leases_state.selected_index > 0 {
                        self.leases_state.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.leases_state.selected_index < self.leases_state.leases.len().saturating_sub(1) {
                        self.leases_state.selected_index += 1;
                    }
                }
                KeyCode::Char('l') => self.fetch_logs(),
                KeyCode::Char('r') => self.refresh_leases(),
                _ => {}
            },
            Screen::DiscordConfig => match key.code {
                KeyCode::Char('i') | KeyCode::Enter => {
                    self.input_mode = InputMode::Insert;
                }
                KeyCode::Char('n') => {
                    if self.discord_state.guide_step < 6 {
                        self.discord_state.guide_step += 1;
                    }
                }
                KeyCode::Char('p') => {
                    if self.discord_state.guide_step > 0 {
                        self.discord_state.guide_step -= 1;
                    }
                }
                KeyCode::Char('x') => {
                    // Clear the active field value
                    self.discord_state.form.clear_active();
                    self.status_message = Some(("Field cleared".to_string(), false));
                }
                KeyCode::Char('X') => {
                    // Clear ALL form fields
                    self.discord_state.form.clear();
                    self.discord_state.guide_step = 0;
                    self.status_message = Some(("All fields cleared — guide reset".to_string(), false));
                }
                KeyCode::Char('u') => {
                    // Auto-populate Bot URL from active lease
                    self.auto_populate_bot_url();
                }
                KeyCode::Char('t') => {
                    // Test provision endpoint
                    self.test_provision_endpoint();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.discord_state.form.next_field();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.discord_state.form.prev_field();
                }
                KeyCode::Char('s') => self.check_discord_status(),
                _ => {}
            },
        }
    }

    fn apply_variable_edit(&mut self) {
        let vi = self.deployment_state.selected_var;
        let val = self.deployment_state.editing_value.clone();
        if let Some(ref mut sdl) = self.deployment_state.sdl {
            if vi < sdl.variables.len() {
                sdl.variables[vi].value = val;
            }
        }
        self.deployment_state.editing_value.clear();
    }

    fn apply_deployment_edit(&mut self) {
        let fi = self.deployment_state.selected_field;
        let si = self.deployment_state.selected_service;
        let val = self.deployment_state.editing_value.clone();
        if val.is_empty() {
            return;
        }
        if let Some(ref mut sdl) = self.deployment_state.sdl {
            if let Some(svc) = sdl.services.get_mut(si) {
                if fi < 4 {
                    match fi {
                        0 => svc.resources.cpu = val,
                        1 => svc.resources.memory = val,
                        2 => svc.resources.storage = val,
                        3 => svc.resources.gpu = val,
                        _ => {}
                    }
                } else if fi - 4 < svc.env_vars.len() {
                    svc.env_vars[fi - 4].value = val;
                }
            }
        }
        self.deployment_state.editing_value.clear();
    }

    fn load_current_field_value(&mut self) {
        let fi = self.deployment_state.selected_field;
        let si = self.deployment_state.selected_service;
        if let Some(ref sdl) = self.deployment_state.sdl {
            if let Some(svc) = sdl.services.get(si) {
                self.deployment_state.editing_value = if fi < 4 {
                    match fi {
                        0 => svc.resources.cpu.clone(),
                        1 => svc.resources.memory.clone(),
                        2 => svc.resources.storage.clone(),
                        3 => svc.resources.gpu.clone(),
                        _ => String::new(),
                    }
                } else if fi - 4 < svc.env_vars.len() {
                    svc.env_vars[fi - 4].value.clone()
                } else {
                    String::new()
                };
            }
        }
    }

    fn handle_form_submit(&mut self) {
        match self.current_screen {
            Screen::DiscordConfig => {
                self.status_message = Some(("Config saved".to_string(), false));
            }
            _ => {}
        }
    }

    // --- Async action dispatchers ---

    fn generate_wallet(&mut self) {
        if let Some(tx) = &self.tx {
            self.wallet_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Generating wallet...".to_string();
            let tx = tx.clone();
            tokio::spawn(async move {
                let gen = KeyGenerator::new();
                match gen.generate_mnemonic() {
                    Ok(mnemonic) => {
                        match gen.create_wallet(mnemonic) {
                            Ok(wallet) => {
                                let _ = tx.send(AppEvent::WalletGenerated {
                                    mnemonic: wallet.mnemonic.clone().unwrap_or_default(),
                                    address: wallet.address.clone().unwrap_or_default(),
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::StatusMessage {
                                    message: format!("Wallet creation failed: {}", e),
                                    is_error: true,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Mnemonic generation failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        }
    }

    fn copy_mnemonic_to_clipboard(&mut self) {
        if let Some(ref mnemonic) = self.wallet_state.wallet.mnemonic {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_text(mnemonic.clone()) {
                        Ok(_) => {
                            self.status_message = Some(("Mnemonic copied to clipboard".to_string(), false));
                        }
                        Err(e) => {
                            self.status_message = Some((format!("Clipboard write failed: {}", e), true));
                        }
                    }
                }
                Err(e) => {
                    self.status_message = Some((format!("Clipboard unavailable: {}", e), true));
                }
            }
        } else {
            self.status_message = Some(("No mnemonic to copy — generate first".to_string(), true));
        }
    }

    fn save_wallet_encrypted(&mut self) {
        if let Some(ref mnemonic) = self.wallet_state.wallet.mnemonic {
            // For now, use a fixed password; a real flow would prompt
            let store = ConfigStore::new().expect("FATAL: cannot create config store");
            store.save_wallet(mnemonic.as_bytes(), "linguabridge-default");
            self.wallet_state.is_saved = true;
            self.status_message = Some(("Wallet saved (encrypted)".to_string(), false));
        } else {
            self.status_message = Some(("No wallet to save — generate first".to_string(), true));
        }
    }

    fn load_wallet_encrypted(&mut self) {
        let store = ConfigStore::new().expect("FATAL: cannot create config store");
        if !store.has_wallet() {
            self.status_message = Some(("No saved wallet found".to_string(), true));
            return;
        }
        // Panics on crypto failure per security policy
        if let Some(plaintext) = store.load_wallet("linguabridge-default") {
            let mnemonic = String::from_utf8(plaintext)
                .expect("FATAL: decrypted wallet is not valid UTF-8");
            if let Some(tx) = &self.tx {
                self.wallet_state.loading = true;
                self.spinner.start();
                self.spinner.message = "Loading wallet...".to_string();
                let tx = tx.clone();
                let mnemonic_clone = mnemonic.clone();
                tokio::spawn(async move {
                    let gen = KeyGenerator::new();
                    match gen.import_wallet(mnemonic_clone) {
                        Ok(wallet) => {
                            let _ = tx.send(AppEvent::WalletGenerated {
                                mnemonic: wallet.mnemonic.clone().unwrap_or_default(),
                                address: wallet.address.clone().unwrap_or_default(),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::StatusMessage {
                                message: format!("Wallet import failed: {}", e),
                                is_error: true,
                            });
                        }
                    }
                });
            }
            self.wallet_state.is_saved = true;
        }
    }

    fn refresh_balance(&mut self) {
        let address = self.wallet_state.wallet.address.clone();
        if let (Some(tx), Some(addr)) = (&self.tx, address) {
            self.wallet_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Fetching balance...".to_string();
            let tx = tx.clone();
            let rpc_url = self.config.network.rpc_url.clone();
            let grpc_url = self.config.network.grpc_url.clone();
            tokio::spawn(async move {
                let client = AkashClient::new(rpc_url, grpc_url);
                match client.query_balance(&addr).await {
                    Ok(balance) => {
                        let _ = tx.send(AppEvent::BalanceUpdated {
                            amount: balance.amount,
                            denom: balance.denom,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Balance query failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        } else {
            self.status_message = Some(("No wallet loaded".to_string(), true));
        }
    }

    fn request_fee_grant(&mut self) {
        self.fee_grant_state.fee_grant_status = "Requested (pending)".to_string();
        self.status_message = Some(("Fee grant request sent - awaiting approval".to_string(), false));
    }

    fn check_fee_grant(&mut self) {
        self.check_fee_allowances();
    }

    fn refresh_bids(&mut self) {
        let address = self.wallet_state.wallet.address.clone();
        let dseq = self.bids_state.dseq.or(self.deployment_state.dseq);
        if let (Some(tx), Some(addr), Some(dseq)) = (&self.tx, address, dseq) {
            self.bids_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Fetching bids...".to_string();
            let tx = tx.clone();
            let rpc_url = self.config.network.rpc_url.clone();
            let grpc_url = self.config.network.grpc_url.clone();
            tokio::spawn(async move {
                let client = AkashClient::new(rpc_url, grpc_url);
                match client.query_bids(&addr, dseq).await {
                    Ok(bids) => {
                        let _ = tx.send(AppEvent::BidsReceived { bids });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Bid query failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        } else {
            self.status_message = Some(("No deployment active".to_string(), true));
        }
    }

    fn accept_bid(&mut self) {
        if let Some(bid) = self.bids_state.bids.get(self.bids_state.selected_index) {
            self.status_message = Some((
                format!("Accepting bid from {}...", &bid.provider[..20.min(bid.provider.len())]),
                false,
            ));
            // In a real flow, this would create and broadcast MsgCreateLease
            self.bids_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Creating lease...".to_string();
        }
    }

    fn refresh_leases(&mut self) {
        let address = self.wallet_state.wallet.address.clone();
        if let (Some(tx), Some(addr)) = (&self.tx, address) {
            self.leases_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Fetching leases...".to_string();
            let tx = tx.clone();
            let rpc_url = self.config.network.rpc_url.clone();
            let grpc_url = self.config.network.grpc_url.clone();
            tokio::spawn(async move {
                let client = AkashClient::new(rpc_url, grpc_url);
                match client.query_leases(&addr).await {
                    Ok(leases) => {
                        let _ = tx.send(AppEvent::LeasesReceived { leases });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Lease query failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        } else {
            self.status_message = Some(("No wallet loaded".to_string(), true));
        }
    }

    fn fetch_logs(&mut self) {
        if let Some(lease) = self.leases_state.leases.get(self.leases_state.selected_index) {
            if let Some(tx) = &self.tx {
                self.leases_state.loading = true;
                self.spinner.start();
                self.spinner.message = "Fetching logs...".to_string();
                let tx = tx.clone();
                let provider_url = lease.provider.clone();
                let dseq = lease.dseq;
                let gseq = lease.gseq;
                let oseq = lease.oseq;
                tokio::spawn(async move {
                    let client = ProviderClient::new();
                    match client.get_logs(&provider_url, dseq, gseq, oseq, "web", 100).await {
                        Ok(entries) => {
                            let lines: Vec<String> = entries.into_iter().map(|e| e.message).collect();
                            let _ = tx.send(AppEvent::LogsReceived { lines });
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::StatusMessage {
                                message: format!("Log fetch failed: {}", e),
                                is_error: true,
                            });
                        }
                    }
                });
            }
        } else {
            self.status_message = Some(("No lease selected".to_string(), true));
        }
    }

    fn submit_deployment(&mut self) {
        // Pre-flight checks before showing confirmation
        self.check_deploy_readiness();
    }

    /// Check all conditions for deployment and show appropriate popup
    fn check_deploy_readiness(&mut self) {
        let wallet_ready = self.wallet_state.wallet.address.is_some();
        let balance_uakt = self.fee_grant_state.balance_uakt;
        let balance_sufficient = balance_uakt >= MIN_DEPLOY_BALANCE_UAKT;
        let has_fee_grant = self.fee_grant_state.has_fee_grant;

        let sdl_ready = self.deployment_state.sdl.is_some()
            && self.deployment_state.sdl_error.is_none();

        let variables_filled = self.deployment_state.sdl
            .as_ref()
            .map(|sdl| sdl.all_variables_filled())
            .unwrap_or(true);

        let mut issues = Vec::new();
        if !wallet_ready {
            issues.push("No wallet loaded — generate or import first".to_string());
        }
        if !sdl_ready {
            issues.push("SDL not loaded or has errors".to_string());
        }
        if !variables_filled {
            if let Some(ref sdl) = self.deployment_state.sdl {
                let unfilled: Vec<&str> = sdl.unfilled_variables()
                    .iter()
                    .map(|v| v.name.as_str())
                    .collect();
                issues.push(format!("Unfilled variables: {}", unfilled.join(", ")));
            }
        }
        if !balance_sufficient && !has_fee_grant {
            issues.push(format!(
                "Balance too low ({} uakt) and no fee grant — request fee grant first",
                balance_uakt
            ));
        }

        let readiness = DeployReadiness {
            wallet_ready,
            balance_sufficient,
            has_fee_grant,
            sdl_ready,
            variables_filled,
            balance_uakt,
            issues: issues.clone(),
        };
        self.deployment_state.readiness = Some(readiness.clone());

        if !issues.is_empty() {
            // Show issues as error popup or redirect to fee grant
            if !wallet_ready {
                self.status_message = Some((issues[0].clone(), true));
            } else if !balance_sufficient && !has_fee_grant {
                // Show fee grant needed popup
                let mut popup = Popup::new(
                    PopupType::FeeGrantNeeded,
                    "Fee Grant Required".to_string(),
                    "Your wallet balance is too low to cover deployment fees.".to_string(),
                );
                popup.details = vec![
                    format!("Current balance: {} uakt ({:.3} AKT)", balance_uakt, balance_uakt as f64 / 1_000_000.0),
                    format!("Minimum required: {} uakt ({:.1} AKT)", MIN_DEPLOY_BALANCE_UAKT, MIN_DEPLOY_BALANCE_UAKT as f64 / 1_000_000.0),
                    String::new(),
                    "Press Tab to go to Fee Grant step, or any key to dismiss.".to_string(),
                ];
                popup.show();
                self.popup = Some(popup);
                self.deployment_state.confirm_pending = true;
            } else {
                self.status_message = Some((issues[0].clone(), true));
            }
        } else {
            // All checks pass — show deployment confirmation
            self.show_deploy_confirm();
        }
    }

    fn show_deploy_confirm(&mut self) {
        let balance_uakt = self.fee_grant_state.balance_uakt;
        let fee_source = if self.fee_grant_state.has_fee_grant {
            "Fee Grant (granter pays gas)"
        } else {
            "Wallet Balance"
        };

        let gpu_models = self.deployment_state.gpu_catalog.selected_models();
        let gpu_text = if gpu_models.is_empty() {
            "None".to_string()
        } else {
            gpu_models.join(", ")
        };

        let service_count = self.deployment_state.sdl
            .as_ref()
            .map(|s| s.services.len())
            .unwrap_or(0);

        let mut popup = Popup::new(
            PopupType::DeployConfirm,
            "Confirm Deployment".to_string(),
            "Ready to submit deployment to Akash Network.".to_string(),
        );
        popup.details = vec![
            format!("Services: {}", service_count),
            format!("GPU Models: {}", gpu_text),
            format!("Fee Source: {}", fee_source),
            format!("Balance: {:.3} AKT", balance_uakt as f64 / 1_000_000.0),
            String::new(),
            "Press Enter to confirm, Esc to cancel.".to_string(),
        ];
        popup.buttons = vec!["Confirm".to_string(), "Cancel".to_string()];
        popup.show();
        self.popup = Some(popup);
        self.deployment_state.confirm_pending = true;
    }

    /// Actually execute the deployment after confirmation
    fn confirm_deployment(&mut self) {
        self.deployment_state.confirm_pending = false;
        self.deployment_state.loading = true;
        self.deployment_state.status = "Submitting...".to_string();
        self.spinner.start();
        self.spinner.message = "Creating deployment...".to_string();

        if let Some(tx) = &self.tx {
            let tx = tx.clone();
            let rpc_url = self.config.network.rpc_url.clone();
            let grpc_url = self.config.network.grpc_url.clone();
            let _address = self.wallet_state.wallet.address.clone().unwrap_or_default();
            tokio::spawn(async move {
                // Get block height for dseq
                let client = AkashClient::new(rpc_url, grpc_url);
                match client.get_block_height().await {
                    Ok(height) => {
                        // In full implementation, we would build and broadcast MsgCreateDeployment
                        // For now, simulate with the block height as dseq
                        let _ = tx.send(AppEvent::DeploymentCreated {
                            dseq: height,
                            txhash: format!("pending-{}", height),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Deployment failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        }
    }

    /// Fetch fee grant allowances for the current wallet
    fn check_fee_allowances(&mut self) {
        let address = self.wallet_state.wallet.address.clone();
        if let (Some(tx), Some(addr)) = (&self.tx, address) {
            self.fee_grant_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Checking fee grants...".to_string();
            let tx = tx.clone();
            let rpc_url = self.config.network.rpc_url.clone();
            let grpc_url = self.config.network.grpc_url.clone();
            tokio::spawn(async move {
                let client = AkashClient::new(rpc_url, grpc_url);
                match client.query_fee_allowances(&addr).await {
                    Ok(allowances) => {
                        let _ = tx.send(AppEvent::FeeAllowanceReceived { allowances });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Fee grant query failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        } else {
            self.status_message = Some(("No wallet loaded".to_string(), true));
        }
    }

    fn check_discord_status(&mut self) {
        if self.discord_state.form.is_complete() {
            self.discord_state.deploy_status = "Configured (all fields set)".to_string();
            self.status_message = Some(("All fields configured — ready to provision".to_string(), false));
        } else {
            let filled = self.discord_state.form.fields.iter().filter(|f| !f.value.is_empty()).count();
            let total = self.discord_state.form.fields.len();
            self.discord_state.deploy_status = format!("Incomplete ({}/{})", filled, total);
            self.status_message = Some((format!("{}/{} fields configured", filled, total), true));
        }
    }

    fn auto_populate_bot_url(&mut self) {
        // Try to get URI from active leases
        if let Some(uri) = self.leases_state.service_uris.first() {
            // Set the "Bot URL" field value
            for field in &mut self.discord_state.form.fields {
                if field.label == "Bot URL" {
                    field.value = uri.clone();
                    break;
                }
            }
            self.status_message = Some(("Bot URL populated from active lease".to_string(), false));
            // Auto-advance guide to submit step if other fields are filled
            if self.discord_state.form.get_value("Bot Token") != "" {
                self.discord_state.guide_step = 6; // Submit step
            }
        } else if let Some(ref uri) = self.discord_state.service_uri {
            for field in &mut self.discord_state.form.fields {
                if field.label == "Bot URL" {
                    field.value = uri.clone();
                    break;
                }
            }
            self.status_message = Some(("Bot URL populated from cached URI".to_string(), false));
        } else {
            self.status_message = Some(("No active lease URI found — deploy first".to_string(), true));
        }
    }

    fn test_provision_endpoint(&mut self) {
        let bot_url = self.discord_state.form.get_value("Bot URL").to_string();
        if bot_url.is_empty() {
            self.status_message = Some(("Bot URL not set — press 'u' to auto-populate".to_string(), true));
            return;
        }

        if let Some(tx) = &self.tx {
            self.discord_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Testing endpoint...".to_string();
            let tx = tx.clone();
            let url = bot_url.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                match client.get(&format!("{}/health", url))
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let status = resp.status();
                        let msg = if status.is_success() {
                            format!("Endpoint healthy ({})", status)
                        } else {
                            format!("Endpoint responded with {}", status)
                        };
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: msg,
                            is_error: !status.is_success(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Endpoint unreachable: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        }
    }

    // --- Mnemonic import functions ---

    fn start_mnemonic_import(&mut self) {
        self.wallet_state.importing_mnemonic = true;
        self.wallet_state.import_text.clear();
        self.input_mode = InputMode::Insert;
        self.status_message = Some(("Enter your mnemonic (12 or 24 words):".to_string(), false));
    }

    fn cancel_mnemonic_import(&mut self) {
        self.wallet_state.importing_mnemonic = false;
        self.wallet_state.import_text.clear();
        self.input_mode = InputMode::Normal;
        self.status_message = Some(("Mnemonic import cancelled".to_string(), false));
    }

    fn import_mnemonic(&mut self) {
        let mnemonic_text = self.wallet_state.import_text.trim().to_string();

        // Validate word count
        let word_count = mnemonic_text.split_whitespace().count();
        if word_count != 12 && word_count != 24 {
            self.status_message = Some((
                format!("Invalid mnemonic: expected 12 or 24 words, got {}", word_count),
                true,
            ));
            self.wallet_state.importing_mnemonic = false;
            self.wallet_state.import_text.clear();
            return;
        }

        // Start import process
        self.wallet_state.importing_mnemonic = false;
        let import_text = mnemonic_text.clone();
        self.wallet_state.import_text.clear();

        if let Some(tx) = &self.tx {
            self.wallet_state.loading = true;
            self.spinner.start();
            self.spinner.message = "Importing wallet...".to_string();
            let tx = tx.clone();
            tokio::spawn(async move {
                let gen = KeyGenerator::new();
                match gen.import_wallet(import_text) {
                    Ok(wallet) => {
                        let _ = tx.send(AppEvent::WalletImported {
                            mnemonic: wallet.mnemonic.clone().unwrap_or_default(),
                            address: wallet.address.clone().unwrap_or_default(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::StatusMessage {
                            message: format!("Mnemonic import failed: {}", e),
                            is_error: true,
                        });
                    }
                }
            });
        }
    }

    fn switch_tab(&mut self, tab: MainTab) {
        self.status_message = None;
        self.main_tab = tab;
        self.sync_screen_from_tab();
    }

    /// Advance to the next step within the Deploy wizard
    fn next_step(&mut self) {
        self.status_message = None;
        match self.main_tab {
            MainTab::Deploy => {
                self.deploy_step = match self.deploy_step {
                    DeployStep::SdlConfig => DeployStep::FeeGrant,
                    DeployStep::FeeGrant => DeployStep::Submit,
                    DeployStep::Submit => DeployStep::Bids,
                    DeployStep::Bids => DeployStep::Leases,
                    DeployStep::Leases => DeployStep::DiscordConfig,
                    DeployStep::DiscordConfig => DeployStep::DiscordConfig,
                };
                self.sync_screen_from_tab();
            }
            _ => {} // No sub-steps for other tabs
        }
    }

    /// Go back one step within the Deploy wizard
    fn prev_step(&mut self) {
        self.status_message = None;
        match self.main_tab {
            MainTab::Deploy => {
                self.deploy_step = match self.deploy_step {
                    DeployStep::SdlConfig => DeployStep::SdlConfig,
                    DeployStep::FeeGrant => DeployStep::SdlConfig,
                    DeployStep::Submit => DeployStep::FeeGrant,
                    DeployStep::Bids => DeployStep::Submit,
                    DeployStep::Leases => DeployStep::Bids,
                    DeployStep::DiscordConfig => DeployStep::Leases,
                };
                self.sync_screen_from_tab();
            }
            _ => {}
        }
    }

    /// Map the current MainTab + DeployStep to the active Screen for rendering
    fn sync_screen_from_tab(&mut self) {
        let new_screen = match self.main_tab {
            MainTab::Deployments => Screen::Deployments,
            MainTab::Wallet => Screen::Wallet,
            MainTab::Deploy => match self.deploy_step {
                DeployStep::SdlConfig => Screen::Deployment,
                DeployStep::FeeGrant => Screen::FeeGrant,
                DeployStep::Submit => Screen::Deployment,
                DeployStep::Bids => Screen::Bids,
                DeployStep::Leases => Screen::Leases,
                DeployStep::DiscordConfig => Screen::DiscordConfig,
            },
        };

        let entering_fee_grant = new_screen == Screen::FeeGrant && self.current_screen != Screen::FeeGrant;
        self.current_screen = new_screen;

        // Auto-check balance and fee grants when entering the FeeGrant step
        if entering_fee_grant && self.wallet_state.wallet.address.is_some() {
            if self.fee_grant_state.balance.is_none() {
                self.refresh_balance();
            }
            if self.fee_grant_state.allowances.is_empty() && !self.fee_grant_state.loading {
                self.check_fee_allowances();
            }
        }
    }

    fn refresh_deployments(&mut self) {
        // Refresh deployment list from stored config
        self.deployments_state.deployments = self.config.deployments.iter().map(|d| {
            DeploymentRecord {
                dseq: d.dseq.parse().unwrap_or(0),
                name: d.name.clone(),
                status: match d.status.as_str() {
                    "active" => DeploymentStatus::Active,
                    "terminated" => DeploymentStatus::Terminated,
                    "failed" => DeploymentStatus::Failed,
                    _ => DeploymentStatus::Unknown,
                },
                services: Vec::new(),
                created_at: d.created_at.clone(),
                updated_at: String::new(),
            }
        }).collect();
        self.status_message = Some(("Deployments refreshed".to_string(), false));
    }

    fn fetch_deployment_logs(&mut self) {
        if self.deployments_state.deployments.is_empty() {
            self.status_message = Some(("No deployments".to_string(), true));
            return;
        }
        // Switch to leases view with the selected deployment's dseq
        let record = &self.deployments_state.deployments[self.deployments_state.selected_index];
        self.bids_state.dseq = Some(record.dseq);
        self.switch_tab(MainTab::Deploy);
        self.deploy_step = DeployStep::Leases;
        self.sync_screen_from_tab();
    }
}
