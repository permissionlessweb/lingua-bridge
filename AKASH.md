
# Akash Network Deployment

LinguaBridge is designed for deployment on Akash Network, a decentralized cloud platform. The secure admin provisioning system ensures your Discord token and API keys are never exposed to Akash providers.

---

## Step 1: Prepare Your Environment

**Generate Admin Keypair:**

```bash
cargo run -p admin-cli --release -- keygen
```

This creates two files:

- `admin.key` - Your private key (keep this secret!)
- `admin.pub` - Your public key (safe to share)

**Save your public key:**

```bash
cat admin.pub
# Copy this value - you'll need it for deploy.yaml
```

**Build and Push Docker Images:**

See the [Building & Publishing Docker Images](#building--publishing-docker-images) section above.

---

## Step 2: Configure Your SDL

Edit `deploy.yaml` and update these placeholders:

**1. Admin Public Key:**

```yaml
env:
  - "LINGUABRIDGE_ADMIN__PUBLIC_KEY=<YOUR_ADMIN_PUBLIC_KEY>"
```

Replace `<YOUR_ADMIN_PUBLIC_KEY>` with the contents of `admin.pub`.

**2. Docker Registry Credentials:**

```yaml
credentials:
  host: ghcr.io
  username: <GHCR_USERNAME>  # Your GitHub username
  password: <GHCR_PAT>       # Your GitHub Personal Access Token
```

**3. Image References:**

```yaml
inference:
  image: ghcr.io/YOUR_ORG/linguabridge-unified:v0.3.0
bot:
  image: ghcr.io/YOUR_ORG/linguabridge-bot:v0.3.0
```

Update `YOUR_ORG` to match your GitHub organization or username.

**4. Optional: HuggingFace Token (for diarization):**

```yaml
env:
  - "HF_TOKEN=<HF_TOKEN>"  # Optional: for speaker diarization
```

---

## Step 3: Deploy to Akash

**Option A: Using Akash Console (Recommended for beginners)**

1. Go to [https://console.akash.network](https://console.akash.network)
2. Connect your wallet
3. Click "Deploy" → "Build Your Template"
4. Upload your `deploy.yaml`
5. Review pricing and click "Create Deployment"
6. Select a provider bid
7. **Save the deployment URI** - you'll need it for provisioning!

**Option B: Using Akash CLI**

```bash
# Create deployment
akash tx deployment create deploy.yaml --from your-wallet --node https://rpc.akashnet.net:443

# Get bids (wait ~30 seconds)
akash query market bid list --owner <your-address> --node https://rpc.akashnet.net:443

# Accept a bid
akash tx market lease create \
  --dseq <deployment-sequence> \
  --provider <provider-address> \
  --from your-wallet \
  --node https://rpc.akashnet.net:443

# Get lease status and URI
akash provider lease-status \
  --dseq <deployment-sequence> \
  --provider <provider-address> \
  --from your-wallet \
  --node https://rpc.akashnet.net:443
```

---



#### Step 4: Get Your Deployment Endpoints

After deployment completes, you'll receive endpoint URIs. Look for:

**Admin Provisioning Endpoint (Port 9999):**

```
https://your-deployment.provider.com:9999
```

**Web Interface (Port 80):**

```
https://your-deployment.provider.com
```

**Example from Akash Console:**

```
Forwarded Ports:
  80/tcp    -> https://lingua.provider1.akash.network
  9999/tcp  -> https://lingua.provider1.akash.network:9999
```

Save these URLs!

---

#### Step 5: Provision the Bot with Discord Token

Now that your deployment is live, provision it with your Discord bot token:

```bash
cargo run -p admin-cli --release -- provision \
  --bot-url https://lingua.provider1.akash.network:9999 \
  --discord-token "YOUR_DISCORD_BOT_TOKEN" \
  --admin-key admin.key
```

Replace:

- `lingua.provider1.akash.network:9999` with your actual admin endpoint
- `YOUR_DISCORD_BOT_TOKEN` with your Discord token from the Developer Portal

**Expected output:**

```
[INFO] Fetching bot's public key from https://lingua.provider1.akash.network:9999/pubkey
[INFO] Encrypting secrets...
[INFO] Signing payload...
[INFO] Sending provisioning request...
[SUCCESS] Bot provisioned successfully!
```

---

#### Step 6: Test Your Deployment

**1. Check Deployment Status:**

```bash
# Check if bot is provisioned
curl https://lingua.provider1.akash.network:9999/status

# Expected response:
# {"status":"provisioned"}
```

**2. Check Deployment Logs (Akash Console):**

Go to your deployment in Akash Console → "Logs" tab:

```
[bot]: [INFO] Secrets provisioned successfully!
[bot]: [INFO] Starting Discord bot...
[bot]: [INFO] Connected to Discord as LinguaBridge#1234
[bot]: [INFO] Serving 15 guilds
[inference]: [INFO] Unified Inference Service starting
[inference]: [INFO] Loading TranslateGemma model: google/translategemma-4b-it
[inference]: [INFO] Model loaded successfully
```

**3. Test Discord Commands:**

In a Discord server where you've added the bot:

```
/ping
```

Expected response: `Pong! Latency: XX ms`

```
/translate
```

Expected: Opens a modal to translate text

**4. Test Voice Translation (Optional):**

1. Join a voice channel
2. Use command: `/join`
3. Speak in the channel
4. Check web interface: `https://lingua.provider1.akash.network/voice/{guild_id}/{channel_id}`

**5. Check Health Endpoints:**

```bash
# Web server health
curl https://lingua.provider1.akash.network/health
# Expected: {"status":"ok"}

# Inference service health (internal, check logs or via bot commands)
```

---

#### Step 7: Update Service URLs (If Using Custom Domain)

If you want to use a custom domain instead of the Akash-assigned URI:

**1. Add DNS Records:**

```
CNAME lingua.yourdomain.com -> lingua.provider1.akash.network
```

**2. Update deploy.yaml:**

```yaml
env:
  # Update these if using custom domain
  - "LINGUABRIDGE_INFERENCE__URL=http://inference.yourdomain.com:8000"
  - "LINGUABRIDGE_VOICE__URL=ws://inference.yourdomain.com:8000/voice"
  - "LINGUABRIDGE_WEB__PUBLIC_URL=https://lingua.yourdomain.com"
```

**3. Redeploy:**

Close the old deployment and create a new one with the updated SDL.

---

#### Troubleshooting Akash Deployment

**No bids received:**

- Check GPU availability: A100/H100 GPUs are in high demand
- Increase pricing in `deploy.yaml` (try 200,000+ uakt for 2x A100)
- Verify SDL syntax: `akash deployment validate deploy.yaml`
- Check provider requirements match your SDL

**Bot won't start (libopus.so.0 error):**

- Rebuild bot image with latest Dockerfile.rust (includes libopus0)
- Push new image and update deploy.yaml image tag

**Provisioning fails:**

```bash
# Verify admin public key matches
cat admin.pub

# Check it matches deploy.yaml
grep ADMIN__PUBLIC_KEY deploy.yaml

# Test connectivity
curl https://your-deployment:9999/pubkey
```

**Bot connects but doesn't respond:**

- Check logs for errors
- Verify Discord bot token is correct
- Check bot has proper permissions in Discord server
- Test inference service is running (check logs for model loading)

**Deployment logs show "waiting for provisioning":**

- This is normal! The bot waits for you to run the `provision` command
- Run Step 5 above to provision the bot

**Web interface not accessible:**

- Check port 80 is exposed globally in deploy.yaml
- Verify Akash forwarded the port (check Console)
- Try using the direct provider URI

---