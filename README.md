# LinguaBridge

Real-time Discord translation bot powered by Google's TranslateGemma models. Translates messages across 49+ languages with automatic language detection, web-based viewing, and per-user language preferences.

## Features

- **Real-time translation** - Automatically translates messages in configured channels
- **49+ languages** - Powered by Google's TranslateGemma (4B, 12B, or 27B parameters)
- **Automatic language detection** - No need to specify source language
- **User preferences** - Each user sets their preferred language
- **Web viewer** - Browser-based interface for reading translations
- **Secure provisioning** - Cryptographic admin transport protects API keys

---

## Create Your Discord Bot

Before deploying, set up your Discord application:

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **New Application** and name it (e.g., "LinguaBridge")
3. Navigate to **Bot** in the sidebar, then click **Add Bot**
4. Under **Privileged Gateway Intents**, enable:
   - **Message Content Intent** (required to read messages for translation)
5. Click **Reset Token** and copy it somewhere safe (you'll need this for provisioning)
6. Go to **OAuth2 → URL Generator**
7. Under **Scopes**, select:
   - `bot`
   - `applications.commands`
8. Under **Bot Permissions**, select:
   - Read Messages/View Channels
   - Send Messages
   - Embed Links
   - Read Message History
9. Copy the generated URL at the bottom and open it in your browser
10. Select the server to add the bot to and authorize it

Keep your bot token safe - you'll use it during the provisioning step after deployment.

---

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup install stable`)
- Python 3.10+ with pip
- NVIDIA GPU with 8GB+ VRAM (for 4B model) or CPU (slower)
- Discord bot token from [Discord Developer Portal](https://discord.com/developers/applications)

### Step 1: Clone and Build

```bash
git clone https://github.com/yourusername/linguabridge.git
cd linguabridge

# Build the Rust bot and admin CLI
cargo build --release
```

### Step 2: Set Up the Inference Service

```bash
cd inference

# Create virtual environment
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt

# Start the inference server
DEVICE=cuda TRANSLATEGEMMA_MODEL=google/translategemma-4b-it python main.py
```

The inference service runs on `http://localhost:8000`. First startup downloads the model (~8GB for 4B).

### Step 3: Generate Admin Keys

In a new terminal:

```bash
cargo run -p admin-cli --release -- keygen
```

This creates two files:

- `admin.key` - Your private key (keep this secret!)
- `admin.pub` - Your public key (configure the bot with this)

### Step 4: Configure the Bot

Copy the example config:

```bash
cp .env.example .env
```

Edit `.env` and set your admin public key:

```bash
# Paste the contents of admin.pub here
LINGUABRIDGE_ADMIN__PUBLIC_KEY=your_base64_public_key_here
```

### Step 5: Start the Bot

```bash
cargo run --release
```

The bot will start and wait for provisioning. You'll see:

```sh
[INFO] LinguaBridge starting...
[INFO] Admin provisioning server listening on 0.0.0.0:9999
[INFO] Waiting for secrets to be provisioned...
```

### Step 6: Provision Your Discord Token

In another terminal, run:

```bash
cargo run -p admin-cli --release -- provision \
  --bot-url http://localhost:9999 \
  --discord-token "YOUR_DISCORD_BOT_TOKEN" \
  --admin-key admin.key
```

The bot will connect to Discord once provisioned:

```sh
[INFO] Secrets provisioned successfully!
[INFO] Starting Discord bot...
[INFO] Bot is ready!
```

---

## Discord Commands

Once the bot is running, use these slash commands in your Discord server:

| Command | Description |
|---------|-------------|
| `/setup init` | Initialize LinguaBridge for your server |
| `/setup channel #channel enable:true` | Enable translation in a channel |
| `/setup languages en,es,fr` | Set target languages for the server |
| `/setup status` | View current configuration |
| `/translate text:Hello target:es` | Translate text to a specific language |
| `/languages` | List all supported languages |
| `/mylang es` | Set your preferred language |
| `/mypreferences` | View your current preferences |
| `/webview` | Get a link to the web translation viewer |

### Initial Server Setup

1. Run `/setup init` as a server admin
2. Run `/setup channel #general enable:true` for each channel to translate
3. Run `/setup languages en,es,fr,de` to set target languages
4. Users run `/mylang <code>` to set their preferred language

---

## Supported Languages

LinguaBridge supports 49 languages including:

| Code | Language | Code | Language | Code | Language |
|------|----------|------|----------|------|----------|
| ar | Arabic | fr | French | pl | Polish |
| bn | Bengali | de | German | pt | Portuguese |
| bg | Bulgarian | el | Greek | pa | Punjabi |
| ca | Catalan | gu | Gujarati | ro | Romanian |
| zh | Chinese | he | Hebrew | ru | Russian |
| hr | Croatian | hi | Hindi | sr | Serbian |
| cs | Czech | hu | Hungarian | sk | Slovak |
| da | Danish | id | Indonesian | sl | Slovenian |
| nl | Dutch | it | Italian | es | Spanish |
| en | English | ja | Japanese | sv | Swedish |
| et | Estonian | kn | Kannada | ta | Tamil |
| fi | Finnish | ko | Korean | te | Telugu |
| | | lv | Latvian | th | Thai |
| | | lt | Lithuanian | tr | Turkish |
| | | mk | Macedonian | uk | Ukrainian |
| | | ms | Malay | ur | Urdu |
| | | ml | Malayalam | vi | Vietnamese |
| | | mr | Marathi | | |
| | | no | Norwegian | | |
| | | fa | Persian | | |

---

## Building & Publishing Docker Images

Before deploying to Akash or any container platform, you need to build and push the Docker images to a registry. LinguaBridge includes a release script that handles this for both GitHub Container Registry (GHCR) and Docker Hub.

### Prerequisites

1. **Docker** installed and running
2. **Registry accounts**:
   - [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry): Use your GitHub account
   - [Docker Hub](https://hub.docker.com/): Create a free account
3. **Authentication tokens**:
   - GHCR: [Personal Access Token](https://github.com/settings/tokens) with `write:packages` scope
   - Docker Hub: Your Docker Hub password or access token

### Using the Release Script

```bash
# Set your usernames
export GITHUB_USER="your-github-username"
export DOCKERHUB_USER="your-dockerhub-username"

# Build and push to both registries with a version tag
./scripts/release.sh --tag v1.0.0 --all

# Or push to just one registry
./scripts/release.sh --tag v1.0.0 --ghcr
./scripts/release.sh --tag v1.0.0 --dockerhub

# Build only one image
./scripts/release.sh --tag v1.0.0 --bot-only --ghcr
./scripts/release.sh --tag v1.0.0 --inference-only --dockerhub

# Preview what would happen (dry run)
./scripts/release.sh --tag v1.0.0 --all --dry-run
```

### Manual Build (Alternative)

If you prefer to build manually:

```bash
# Build bot image
docker build -f docker/Dockerfile.rust -t linguabridge-bot:v1.0.0 .

# Build inference image
docker build -f docker/Dockerfile.inference -t linguabridge-inference:v1.0.0 .

# Tag for GHCR
docker tag linguabridge-bot:v1.0.0 ghcr.io/YOUR_USER/linguabridge-bot:v1.0.0
docker tag linguabridge-inference:v1.0.0 ghcr.io/YOUR_USER/linguabridge-inference:v1.0.0

# Login and push
docker login ghcr.io -u YOUR_USER
docker push ghcr.io/YOUR_USER/linguabridge-bot:v1.0.0
docker push ghcr.io/YOUR_USER/linguabridge-inference:v1.0.0
```

### Update deploy.yaml

After pushing, update `deploy.yaml` with your image references:

```yaml
services:
  inference:
    image: ghcr.io/YOUR_USER/linguabridge-inference:v1.0.0
  bot:
    image: ghcr.io/YOUR_USER/linguabridge-bot:v1.0.0
```

---

## Docker Deployment

### Local Development with Docker Compose

```bash
cd docker

# Set your admin public key
export ADMIN_PUBLIC_KEY="your_base64_public_key_here"

# Build and start
docker compose up -d
```

### Provision the Running Container

```bash
# From your local machine with the admin.key file
cargo run -p admin-cli --release -- provision \
  --bot-url http://your-server:9999 \
  --discord-token "YOUR_DISCORD_BOT_TOKEN" \
  --admin-key admin.key
```

### Akash Network Deployment

LinguaBridge includes an Akash SDL file (`deploy.yaml`) for decentralized deployment. The secure admin provisioning system ensures your Discord token and API keys are never exposed to Akash providers.

#### Step 1: Prepare for Deployment

```bash
# Generate your admin keypair
cargo run -p admin-cli --release -- keygen

# Note the public key from admin.pub - you'll need it
cat admin.pub
```

#### Step 2: Configure the SDL

Edit `deploy.yaml`:

1. Replace `<YOUR_ADMIN_PUBLIC_KEY>` with your admin public key
2. Update the Docker image URLs to your registry
3. Optionally customize resource allocations

#### Step 3: Deploy to Akash

```bash
# Using Akash CLI
akash tx deployment create deploy.yaml --from your-wallet

# Or use Akash Console (https://console.akash.network)
# Upload deploy.yaml and follow the guided deployment
```

#### Step 4: Provision After Deployment

Once deployed, note the assigned URI from Akash, then:

```bash
cargo run -p admin-cli --release -- provision \
  --bot-url https://<your-akash-uri>:9999 \
  --discord-token "YOUR_DISCORD_BOT_TOKEN" \
  --admin-key admin.key
```

#### Security Model

The provisioning uses:

- Ed25519 signatures to verify admin identity
- X25519 key exchange for forward secrecy
- ChaCha20-Poly1305 authenticated encryption
- Memory-only storage (secrets never written to disk)

This ensures that even Akash providers with access to your container cannot extract sensitive credentials.

---

## Configuration Reference

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LINGUABRIDGE_ADMIN__PUBLIC_KEY` | (required) | Admin Ed25519 public key (base64) |
| `LINGUABRIDGE_ADMIN__PORT` | `9999` | Admin provisioning endpoint port |
| `LINGUABRIDGE_INFERENCE__URL` | `http://localhost:8000` | Inference service URL |
| `LINGUABRIDGE_INFERENCE__MODEL` | `google/translategemma-4b-it` | TranslateGemma model |
| `LINGUABRIDGE_WEB__PORT` | `3000` | Web server port |
| `LINGUABRIDGE_WEB__PUBLIC_URL` | `http://localhost:3000` | Public URL for links |
| `LINGUABRIDGE_DATABASE__URL` | `sqlite://linguabridge.db` | Database connection |
| `RUST_LOG` | `linguabridge=info` | Log level |

### Model Selection

| Model | VRAM Required | Speed | Quality |
|-------|---------------|-------|---------|
| `google/translategemma-4b-it` | ~8GB | Fast | Good |
| `google/translategemma-12b-it` | ~24GB | Medium | Better |
| `google/translategemma-27b-it` | ~54GB | Slower | Best |

---

## Admin CLI Reference

The admin CLI (`linguabridge-admin`) manages secure provisioning.

### Generate Keypair

```bash
linguabridge-admin keygen [--output-dir ./keys]
```

Creates `admin.key` (private) and `admin.pub` (public).

### Provision Bot

```bash
linguabridge-admin provision \
  --bot-url http://localhost:9999 \
  --discord-token "token" \
  --admin-key admin.key \
  [--hf-token "optional_huggingface_token"] \
  [--custom KEY=VALUE ...]
```

### Check Status

```bash
linguabridge-admin status --bot-url http://localhost:9999
```

### Display Public Key

```bash
linguabridge-admin pubkey --admin-key admin.key
```

---

## Troubleshooting

### Bot won't start

**"Admin public key must be configured"**

Set the `LINGUABRIDGE_ADMIN__PUBLIC_KEY` environment variable to your admin public key (from `admin.pub`).

**"Waiting for secrets to be provisioned..."**

The bot is waiting for you to run the `provision` command with your Discord token.

### Inference service issues

**"CUDA out of memory"**

Use a smaller model or run on CPU:

```bash
DEVICE=cpu python main.py
```

**Model download fails**

Check your Hugging Face access. Some models may require accepting terms:

1. Visit <https://huggingface.co/google/translategemma-4b-it>
2. Accept the license
3. Set `HF_TOKEN` environment variable if needed

### Provisioning fails

**"Invalid signature"**

Make sure you're using the correct admin.key file that matches the public key configured in the bot.

**"Already provisioned"**

The bot only accepts provisioning once per restart. Restart the bot to re-provision.

---

## Architecture

```
                                    +------------------+
                                    |   Discord API    |
                                    +--------+---------+
                                             |
+----------------+                  +--------+---------+
|  Admin CLI     |  provision -->  |  LinguaBridge    |
| (local machine)|                 |  (Rust binary)   |
+----------------+                 +--------+---------+
                                            |
                              +-------------+-------------+
                              |                           |
                     +--------+--------+         +--------+--------+
                     | Inference Svc   |         |   Web Server    |
                     | (Python/FastAPI)|         |   (Axum)        |
                     +-----------------+         +-----------------+
```

---

## Security Model

LinguaBridge uses a cryptographic provisioning system to protect sensitive credentials:

1. **Key Generation**: Admin generates an Ed25519 keypair locally
2. **Trust Anchor**: Only the public key is deployed with the bot
3. **Secure Channel**: Provisioning uses X25519 key exchange + ChaCha20-Poly1305
4. **Forward Secrecy**: Bot generates ephemeral keys on each restart
5. **Memory Only**: Secrets are never written to disk and zeroized on drop

This design ensures that even if an attacker gains access to the running container or deployment configuration, they cannot extract the Discord token or other secrets without the admin's private key.

---

## Development

### Running Tests

```bash
cargo test
```

### Local Development Setup

Terminal 1 - Inference:

```bash
cd inference && python main.py
```

Terminal 2 - Bot:

```bash
cargo run
```

Terminal 3 - Provision:

```bash
cargo run -p admin-cli -- provision --bot-url http://localhost:9999 --discord-token $DISCORD_TOKEN --admin-key admin.key
```

---

## License

MIT

---

<!-- AGENTIC CONTEXT -->
<!-- The following section provides structured information for AI assistants -->

## Agentic Context

This section provides technical details for AI agents working with this codebase.

### Project Structure

```
linguabridge/
├── Cargo.toml              # Workspace root with crypto dependencies
├── admin-cli/              # Secure provisioning CLI tool
│   ├── Cargo.toml
│   └── src/main.rs         # keygen, provision, status, pubkey commands
├── src/
│   ├── main.rs             # Entry point with provisioning wait loop
│   ├── lib.rs              # Library exports
│   ├── config.rs           # Configuration with AdminConfig
│   ├── admin/              # Secure admin transport layer
│   │   ├── mod.rs          # Module exports
│   │   ├── crypto.rs       # Ed25519/X25519/ChaCha20 primitives
│   │   ├── secrets.rs      # In-memory SecretStore with zeroize
│   │   └── transport.rs    # Axum routes for /pubkey, /status, /provision
│   ├── bot/                # Discord bot (serenity/poise)
│   │   ├── mod.rs          # Bot setup with start_bot_with_token()
│   │   ├── handler.rs      # Message event handler
│   │   └── commands/       # Slash commands (setup, translate, mylang, webview)
│   ├── web/                # Web server (axum)
│   │   ├── mod.rs          # Server setup
│   │   ├── routes.rs       # HTTP routes
│   │   └── websocket.rs    # WebSocket for live translations
│   ├── db/                 # Database layer (sqlx/sqlite)
│   │   ├── mod.rs          # Pool and repos
│   │   └── models.rs       # Guild, User, Session models
│   └── translation/        # Translation client
│       ├── mod.rs          # TranslationClient
│       └── cache.rs        # LRU translation cache
├── inference/              # Python inference sidecar
│   ├── main.py             # FastAPI server
│   ├── translator.py       # TranslateGemma wrapper
│   ├── detector.py         # Language detection
│   └── requirements.txt    # Python dependencies
├── docker/
│   ├── docker-compose.yml  # Multi-container deployment
│   ├── Dockerfile.rust     # Bot container
│   └── Dockerfile.inference # Inference container
├── config/
│   └── default.toml        # Default configuration
└── static/                 # Web frontend assets
```

### Key Technical Decisions

1. **Python sidecar for inference**: Candle (Rust ML) doesn't support Gemma 3 architecture yet. Python sidecar uses transformers library.

2. **Secure admin transport**: Environment variables expose secrets on decentralized platforms (Akash). Cryptographic provisioning solves this:
   - Ed25519 for admin authentication
   - X25519 for ephemeral key exchange
   - ChaCha20-Poly1305 for payload encryption
   - zeroize for secure memory cleanup

3. **Memory-only secrets**: SecretsPayload uses manual Drop impl to zeroize all fields. Never persisted to disk.

4. **Boot sequence**: main.rs waits for provisioning before starting Discord bot:

   ```rust
   // 1. Load config (no secrets)
   // 2. Start admin server on port 9999
   // 3. Wait for secrets via secret_store.wait_for_provisioning()
   // 4. Start Discord bot with token from SecretStore
   ```

### Important Files for Modifications

- **Adding new bot commands**: `src/bot/commands/mod.rs` + new command file
- **Adding new secrets**: Update `SecretsPayload` in both `src/admin/secrets.rs` and `admin-cli/src/main.rs`
- **Changing config**: `src/config.rs` and `config/default.toml`
- **Database schema**: `src/db/` with sqlx migrations

### Crypto Implementation Details

Location: `src/admin/crypto.rs`

```rust
// Key types
EphemeralKeyPair          // X25519 session keys (regenerated on boot)
SigningKey / VerifyingKey // Ed25519 admin identity

// Functions
verify_signature()        // Verify admin Ed25519 signature
decrypt_payload()         // X25519 ECDH + ChaCha20-Poly1305
encrypt_payload()         // For responses (admin CLI uses this)
build_signature_message() // Canonical message format for signing
```

### Provisioning Protocol

1. Admin CLI calls `GET /pubkey` to get bot's ephemeral X25519 public key
2. CLI generates ephemeral X25519 keypair
3. CLI computes shared secret via ECDH
4. CLI encrypts secrets with ChaCha20-Poly1305
5. CLI signs (bot_pubkey || cli_pubkey || ciphertext || nonce) with Ed25519
6. CLI sends `POST /provision` with signature + encrypted payload
7. Bot verifies signature against configured admin public key
8. Bot decrypts payload and stores in SecretStore
9. Bot notifies waiters and starts Discord connection

### Dependencies

Key crates:

- `serenity` / `poise` - Discord bot framework
- `axum` - Web server
- `sqlx` - Async database
- `ed25519-dalek` - Ed25519 signatures
- `x25519-dalek` - X25519 key exchange
- `chacha20poly1305` - AEAD encryption
- `zeroize` - Secure memory cleanup

### Common Tasks

**Run the full stack locally:**

```bash
# Terminal 1: Inference
cd inference && python main.py

# Terminal 2: Bot
cargo run

# Terminal 3: Provision
cargo run -p admin-cli -- provision --bot-url http://localhost:9999 --discord-token $TOKEN --admin-key admin.key
```

**Check if bot is provisioned:**

```bash
curl http://localhost:9999/status
# {"status":"waiting_for_provisioning"} or {"status":"provisioned"}
```

**Get bot's ephemeral public key:**

```bash
curl http://localhost:9999/pubkey
# {"public_key":"base64..."}
```
