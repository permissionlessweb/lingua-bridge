# Discord Bot Tester

Automated testing tool for LinguaBridge Discord bot deployment on Akash Network.

## Features

- ✅ Verifies bot is online and connected
- ✅ Checks bot joined target guild
- ✅ Tests backend health endpoint
- ✅ Tests backend provisioning status
- ✅ Registers slash commands (`/ping`, `/translate`)
- ✅ Provides manual test instructions
- ✅ Supports rotating backend providers (just update `.env`)

## Prerequisites

### 1. Create Discord Bot (One-time Setup)

1. Go to https://discord.com/developers/applications
2. Click "New Application" → name it (e.g., "LinguaBridge Bot")
3. Go to "Bot" tab → "Add Bot" → Copy Token (save for `.env`)
4. **Enable Intents:**
   - Bot tab → Privileged Gateway Intents
   - Enable: `SERVER MEMBERS INTENT`, `MESSAGE CONTENT INTENT`
5. Go to "OAuth2" → "URL Generator":
   - Scopes: `bot`, `applications.commands`
   - Bot Permissions:
     - Send Messages
     - Use Slash Commands
     - Connect (for voice)
     - Speak (for voice)
   - Copy generated URL
6. Open URL in browser → Select your test server → Authorize

### 2. Get Guild ID

1. Enable Developer Mode in Discord:
   - User Settings → Advanced → Developer Mode (toggle on)
2. Right-click your server icon → "Copy ID"
3. Save for `.env`

### 3. Deploy Backend to Akash

Follow the main [AKASH.md](../../AKASH.md) to deploy LinguaBridge to Akash. You'll need the admin endpoint URL (port 9999).

## Installation

```bash
cd tools/discord-bot-tester
pip install -r requirements.txt
```

## Configuration

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
nano .env
```

Example `.env`:
```bash
BOT_TOKEN=MTIzNDU2Nzg5MDEyMzQ1Njc4OQ.GhIJKL.mNoPqRsTuVwXyZaBcDeFgHiJkLmNoPqRsTuVw
GUILD_ID=987654321098765432
BACKEND_URL=https://lingua.provider1.akash.network:9999
```

## Usage

### Run Tests

```bash
python bot_tester.py
```

### Expected Output

```
2024-01-24 12:00:00 [INFO] Setting up bot commands...
2024-01-24 12:00:01 [INFO] ✅ Commands registered and synced
2024-01-24 12:00:02 [INFO] Bot connected as LinguaBridge#1234 (ID: 123456789)
2024-01-24 12:00:03 [INFO] ✅ Bot is in guild: My Test Server (ID: 987654321098765432)
2024-01-24 12:00:04 [INFO] Checking health endpoint: https://lingua.provider1.akash.network/health
2024-01-24 12:00:05 [INFO] ✅ Backend health check passed
2024-01-24 12:00:06 [INFO] Checking status endpoint: https://lingua.provider1.akash.network:9999/status
2024-01-24 12:00:07 [INFO] ✅ Bot is provisioned with Discord token

============================================================
MANUAL TEST INSTRUCTIONS
============================================================
1. Open Discord and go to guild: My Test Server
2. In any text channel, type: /ping
   Expected: Bot responds with 'Pong! Bot is alive.'
3. Test translation: /translate text:hello target:es
   Expected: Bot responds with Spanish translation
============================================================

============================================================
AUTOMATED TEST SUMMARY
============================================================
Bot Online: ✅ PASS
Guild Joined: ✅ PASS
Backend Health: ✅ PASS
Backend Status: ✅ PASS
Commands Registered: ✅ PASS
Ping Test: ❌ FAIL  # Manual test required
Translate Test: ❌ FAIL  # Manual test required
============================================================
🎉 All automated tests passed! Bot is configured correctly.
   Run manual tests above to verify full functionality.

🤖 Bot is running. Run slash commands in Discord to test.
Press Ctrl+C to stop.
```

### Manual Testing

After automated tests pass, open Discord and run:

1. **Ping Test:**
   ```
   /ping
   ```
   Expected: "Pong! Bot is alive."

2. **Translation Test:**
   ```
   /translate text:hello target:es
   ```
   Expected: "✅ Translated: hola"

3. **Voice Translation Test:**
   - Join a voice channel
   - Use Discord's voice chat
   - Bot should auto-translate speech

## Rotating Backend Providers

To test a new Akash deployment:

1. Update `.env`:
   ```bash
   BACKEND_URL=https://new-provider.akash.network:9999
   ```

2. Re-run script:
   ```bash
   python bot_tester.py
   ```

Commands automatically point to the new backend. No code changes needed.

## Troubleshooting

### Bot not in guild

**Error:**
```
❌ Bot not in guild 987654321098765432. Invite it first!
```

**Solution:**
1. Go to https://discord.com/developers/applications
2. Select your application → OAuth2 → URL Generator
3. Scopes: `bot`, `applications.commands`
4. Permissions: Send Messages, Use Slash Commands, Connect, Speak
5. Copy URL, open in browser, invite to guild

### Backend unreachable

**Error:**
```
❌ Backend unreachable: Cannot connect to host
```

**Solution:**
1. Verify deployment is running: `akash query deployment get --dseq <DSEQ>`
2. Check logs: `akash logs --dseq <DSEQ> --service bot`
3. Verify endpoint URL in `.env` matches Akash provider URI

### Commands not appearing

**Error:**
Slash commands don't show up in Discord

**Solution:**
1. Check bot has `applications.commands` scope in invite URL
2. Wait 5 seconds after bot connects (syncing takes time)
3. Restart Discord client

### Translation timeout

**Error:**
```
❌ Backend timeout (>30s)
```

**Solution:**
1. Check inference service is running: `akash logs --dseq <DSEQ> --service inference`
2. Verify GPU is allocated (inference needs GPU)
3. Check if models are still loading (first request can be slow)

### Bot not provisioned

**Warning:**
```
⚠️  Bot not provisioned yet. Run: cargo run -p admin-cli -- provision
```

**Solution:**
Run provisioning command:
```bash
cargo run -p admin-cli --release -- provision \
  --bot-url https://lingua.provider1.akash.network:9999 \
  --discord-token "YOUR_DISCORD_BOT_TOKEN" \
  --admin-key admin.key
```

## Integration with CI/CD

Add to your deployment pipeline:

```bash
#!/bin/bash
# After Akash deployment succeeds...

cd tools/discord-bot-tester

# Configure for new deployment
export BOT_TOKEN="$DISCORD_BOT_TOKEN"
export GUILD_ID="$DISCORD_GUILD_ID"
export BACKEND_URL="https://$(akash provider lease-status --dseq $DSEQ | grep -oP 'https://\S+')"

# Run tests
python bot_tester.py

# Check exit code
if [ $? -eq 0 ]; then
  echo "✅ Bot deployment verified"
else
  echo "❌ Bot deployment failed verification"
  exit 1
fi
```

## Security Notes

- **Never commit `.env` to Git** (already in `.gitignore`)
- Rotate bot token if exposed: Discord Developer Portal → Bot → Regenerate Token
- Admin key (`admin.key`) must be kept secret
- Use environment variables in CI/CD, not hardcoded values

## License

Same as main LinguaBridge project.
