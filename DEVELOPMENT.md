# Local Development

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
