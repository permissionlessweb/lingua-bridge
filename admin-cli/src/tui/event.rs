use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::tui::api::{BidInfo, FeeAllowanceInfo, LeaseInfo};

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Quit,
    // Async operation results
    WalletGenerated { mnemonic: String, address: String },
    WalletImported { mnemonic: String, address: String },
    BalanceUpdated { amount: String, denom: String },
    BidsReceived { bids: Vec<BidInfo> },
    LeasesReceived { leases: Vec<LeaseInfo> },
    TxBroadcast { txhash: String, success: bool, message: String },
    StatusMessage { message: String, is_error: bool },
    LogsReceived { lines: Vec<String> },
    FeeAllowanceReceived { allowances: Vec<FeeAllowanceInfo> },
    DeploymentCreated { dseq: u64, txhash: String },
}

/// Event handler for the TUI
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        // Spawn event handling task
        tokio::spawn(async move {
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                let tick_delay = tick_interval.tick();
                let event_delay = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        if tx_clone.send(AppEvent::Tick).is_err() {
                            break;
                        }
                    }
                    maybe_event = event_delay => {
                        match maybe_event {
                            Some(Ok(CrosstermEvent::Key(key))) => {
                                if key.kind == event::KeyEventKind::Press {
                                    if tx_clone.send(AppEvent::Key(key)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(Ok(CrosstermEvent::Resize(_, _))) => {}
                            Some(Err(_)) => {
                                if tx_clone.send(AppEvent::Quit).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// Get the next event
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    /// Get a sender for dispatching async results back to the event loop
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }
}
