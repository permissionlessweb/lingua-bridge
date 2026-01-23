# LinguaBridge Admin CLI

Terminal interface for deploying a LinguaBridge translation bot to Akash Network and connecting it to your Discord server.

## Install

```bash
cargo install --path admin-cli
```

## Launch

```bash
linguabridge-admin tui
```

The TUI opens on a splash screen. Press any key to begin.

## Workflow

The TUI guides you through six screens in order. Navigate forward with `Tab`, back with `Shift+Tab`.

### 1. Wallet

Generate or import a Cosmos wallet for Akash transactions.

| Key | Action |
|-----|--------|
| `g` | Generate a new 24-word mnemonic and derive an `akash1...` address |
| `c` | Copy mnemonic to clipboard (macOS/Linux/Windows) |
| `s` | Save wallet encrypted to disk |
| `l` | Load previously saved wallet |
| `r` | Refresh on-chain balance |

After generating, a popup displays your mnemonic. Copy it to clipboard with `c` or save encrypted with `s`. The wallet is encrypted with AES-256-GCM and stored at `~/.config/linguabridge/wallet.enc`.

Crypto operations (encrypt/decrypt) will **panic on failure** — this is intentional. If your wallet file is corrupted or the password is wrong, the process terminates rather than silently degrading security.

### 2. Fee Grant

Fund your wallet for gas fees.

| Key | Action |
|-----|--------|
| `r` | Request a fee grant from the testnet faucet |
| `c` | Check current fee grant allowance |
| `b` | Refresh wallet balance |

You need a fee grant (or AKT balance) before you can create deployments.

### 3. Deployment

The deployment screen parses and displays the actual `deploy.yaml` SDL file. A bundled default is included; you can also load a custom SDL.

**Layout**: Left panel shows the raw YAML. Right panel shows editable fields for the selected service.

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate between services (inference, voice-inference, bot) |
| `Tab`/`Shift+Tab` | Navigate between editable fields |
| `i` or `Enter` | Enter INSERT mode to edit the selected field |
| `d` | Submit the deployment |
| `Esc` | Exit INSERT mode |

**Editable fields per service:**

- **Resources**: CPU, Memory, Storage, GPU
- **Environment variables**: All `KEY=VALUE` entries from the `env:` section

Changes are applied inline. Press `d` to submit the modified SDL as a `MsgCreateDeployment`.

### 4. Bids

Providers on the network bid to host your deployment.

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate bid list |
| `Enter` | Accept the selected bid (creates a lease) |
| `r` | Refresh bids |

Wait a few seconds after deployment creation for bids to arrive, then press `r`.

### 5. Leases

Monitor active leases and view logs.

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate lease list |
| `l` | Fetch logs for the selected lease |
| `r` | Refresh lease list |

The right panel shows lease details (provider, price, state) and a scrollable log viewer.

### 6. Discord Config

Connect the deployed bot to your Discord server. This screen provides both a step-by-step interactive guide and a full reference checklist.

**Guide navigation:**

| Key | Action |
|-----|--------|
| `n` | Next guide step |
| `p` | Previous guide step |
| `i` | Enter INSERT mode to fill the form |
| `Enter` | Save configuration |
| `s` | Check deployment status |

**Setup steps:**

1. Create a Discord Application at discord.com/developers/applications
2. Create a Bot User and copy the token
3. Set bot permissions (Send Messages, Read Messages, Connect, Speak)
4. Invite the bot to your server via the OAuth2 URL
5. Enter the Bot Token in the form
6. Enter the Bot URL (service URI from your active lease)
7. Submit to provision the bot

**Form fields:**

- **Bot Token** — Your Discord bot token
- **Bot URL** — The service URI from your active lease

## Global Keys

| Key | Action |
|-----|--------|
| `Tab` | Next screen |
| `Shift+Tab` | Previous screen |
| `q` | Quit |
| `Ctrl+C` | Force quit |
| `Esc` | Exit INSERT mode |

## Config Storage

Encrypted wallet is stored at `~/.config/linguabridge/wallet.enc`. The encryption uses AES-256-GCM with a key derived via HKDF-SHA256. Plaintext config (network endpoints, deployment history) is stored alongside at `~/.config/linguabridge/config.json`.

---

## Workspace Overview (Developer Reference)

```
ziggurat/
  Cargo.toml                    # Workspace: linguabridge, admin-cli, linguabridge-types
  deploy.yaml                   # Akash SDL: inference + voice-inference + bot services
  src/                          # Main bot crate: Discord (serenity/poise), Axum, SQLx, voice
  linguabridge-types/           # Shared protobuf types (prost): Akash deployment/market msgs
  admin-cli/                    # This crate
    src/
      main.rs                   # CLI entry: clap Commands::Tui -> tui::run_tui()
      tui/
        mod.rs                  # Terminal setup (crossterm), main loop: draw + handle events
        app.rs                  # App state machine: Screen enum, per-screen state, key dispatch,
                                # async action spawners, SDL editing state
        event.rs                # EventHandler: crossterm key stream + tick timer + AppEvent mpsc
        ui.rs                   # Render dispatcher: header, screen content, footer, overlays
        theme.rs                # AkashTheme: #E53E3E primary, mode colors, style helpers
        input.rs                # InputMode enum: Normal / Insert / Command
        sdl.rs                  # SDL parser: extracts services, env vars, resources from YAML
                                # Bundles deploy.yaml via include_str!, supports custom paths
        screens/
          splash.rs             # ASCII logo, press-any-key
          wallet.rs             # Wallet state, clipboard copy, encrypted save/load status
          fee_grant.rs          # Fee grant status, allowance, balance
          deployment.rs         # SDL YAML viewer (left) + inline field editor (right)
                                # Services list, resource + env var editing per service
          bids.rs               # Dynamic table from BidInfo vec, selection highlight
          leases.rs             # Lease table + details panel + LogViewer widget
          discord_config.rs     # Step-by-step guide (left) + reference checklist (right)
                                # Progress tracking, form for Bot Token + Bot URL
        widgets/
          form.rs               # Interactive multi-field form: input, navigation, render
          popup.rs              # Modal overlay (mnemonic display, confirms, errors)
          spinner.rs            # Braille animation loading indicator
          log_viewer.rs         # Scrollable bounded log line buffer
        api/
          client.rs             # AkashClient: REST queries (balance, bids, leases, broadcast)
          provider.rs           # ProviderClient: manifest submission, status, logs
        wallet/
          keygen.rs             # BIP-39 mnemonic + BIP-32 HD derivation (cosmrs) -> akash1...
          signer.rs             # Transaction signing: SignDoc -> secp256k1 signature bytes
        config/
          schema.rs             # AppConfig, NetworkConfig, WalletConfig, SavedDeployment
          store.rs              # AES-256-GCM encrypted persistence (~/.config/linguabridge/)
                                # Panics on crypto failure (security policy)
```

### Data Flow

1. **User presses a key** -> crossterm EventStream -> `AppEvent::Key` via mpsc channel
2. **App dispatches** -> `handle_screen_key` triggers an action (e.g. `generate_wallet`)
3. **Async work** -> `tokio::spawn` runs wallet/API logic, sends result as `AppEvent` variant
4. **State update** -> `handle_event` matches the result, updates `App` fields, stops spinner
5. **Next frame** -> `ui::render` reads `App` state, draws the current screen with fresh data

### SDL Parsing

The `sdl.rs` module bundles the workspace's `deploy.yaml` via `include_str!` and parses it at startup. It extracts:

- **Services** (name, image, env vars)
- **Resources** from `profiles.compute` (cpu, memory, storage, gpu)

The deployment screen renders the raw YAML on the left and exposes editable fields on the right. Edits to env vars and resources are applied in-memory; pressing `d` submits the modified SDL.

### Security Model

- **Zeroize on drop**: `Wallet` and `KeyPair` scrub private key material from memory.
- **Panic on crypto error**: `ConfigStore::save_wallet` and `load_wallet` use `expect()` — the process terminates rather than leaking partial state.
- **Clipboard**: Uses `arboard` for cross-platform clipboard access. Graceful fallback if clipboard is unavailable.
- **Encrypted at rest**: Mnemonic encrypted with AES-256-GCM, key derived via HKDF-SHA256 from a password.

### Key Design Decisions

- **No blocking in the render loop**: All network/crypto work runs in spawned tasks.
- **Single channel**: One `mpsc::UnboundedSender<AppEvent>` carries both input events and async results.
- **Shared types crate**: `linguabridge-types` holds proto-generated structs for both the main bot and admin-cli.
- **SDL-first deployment**: Instead of a generic form, the actual `deploy.yaml` is displayed and parsed, with only env vars and resources editable inline.
- **Guided Discord setup**: Interactive step-by-step wizard with a persistent reference checklist, replacing the old bare form.
