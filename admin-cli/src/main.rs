//! LinguaBridge Admin CLI
//!
//! Secure provisioning tool for managing LinguaBridge bot instances.
//!
//! Commands:
//! - keygen: Generate admin Ed25519 keypair
//! - provision: Send encrypted secrets to a running bot
//! - status: Check bot provisioning status

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use clap::{Parser, Subcommand};
use colored::Colorize;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroize;

#[derive(Parser)]
#[command(name = "linguabridge-admin")]
#[command(about = "Secure provisioning tool for LinguaBridge", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new admin Ed25519 keypair
    Keygen {
        /// Output directory for keys
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Provision secrets to a running LinguaBridge bot
    Provision {
        /// Bot's admin endpoint URL (e.g., http://localhost:9999)
        #[arg(short, long)]
        bot_url: String,
        /// Path to admin private key file
        #[arg(short, long)]
        admin_key: PathBuf,
        /// Discord bot token
        #[arg(long)]
        discord_token: String,
        /// Hugging Face token (optional)
        #[arg(long)]
        hf_token: Option<String>,
    },
    /// Check bot provisioning status
    Status {
        /// Bot's admin endpoint URL
        #[arg(short, long)]
        bot_url: String,
    },
    /// Display the public key from a private key file
    Pubkey {
        /// Path to admin private key file
        #[arg(short, long)]
        admin_key: PathBuf,
    },
}

/// Secrets payload to send to bot
#[derive(Serialize)]
struct SecretsPayload {
    discord_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hf_token: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    custom: HashMap<String, String>,
}

impl Drop for SecretsPayload {
    fn drop(&mut self) {
        self.discord_token.zeroize();
        if let Some(ref mut token) = self.hf_token {
            token.zeroize();
        }
        for value in self.custom.values_mut() {
            value.zeroize();
        }
        self.custom.clear();
    }
}

/// Provision request to bot
#[derive(Serialize)]
struct ProvisionRequest {
    admin_x25519_public: String,
    ciphertext: String,
    nonce: String,
    signature: String,
}

/// Bot public key response
#[derive(Deserialize)]
struct PublicKeyResponse {
    public_key: String,
}

/// Bot status response
#[derive(Deserialize)]
struct StatusResponse {
    status: String,
}

/// Provision response
#[derive(Deserialize)]
struct ProvisionResponse {
    success: bool,
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { output } => cmd_keygen(output).await,
        Commands::Provision {
            bot_url,
            admin_key,
            discord_token,
            hf_token,
        } => cmd_provision(bot_url, admin_key, discord_token, hf_token).await,
        Commands::Status { bot_url } => cmd_status(bot_url).await,
        Commands::Pubkey { admin_key } => cmd_pubkey(admin_key).await,
    }
}

/// Generate admin keypair
async fn cmd_keygen(output: PathBuf) -> Result<()> {
    println!("{}", "Generating Ed25519 admin keypair...".cyan());

    // Generate signing key
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // Encode keys
    let private_key_bytes = signing_key.to_bytes();
    let public_key_base64 = BASE64.encode(verifying_key.to_bytes());

    // Create output directory if needed
    fs::create_dir_all(&output).context("Failed to create output directory")?;

    // Write private key (keep secure!)
    let private_key_path = output.join("admin.key");
    fs::write(&private_key_path, private_key_bytes).context("Failed to write private key")?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&private_key_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&private_key_path, perms)?;
    }

    // Write public key
    let public_key_path = output.join("admin.pub");
    fs::write(&public_key_path, &public_key_base64).context("Failed to write public key")?;

    println!();
    println!("{}", "Keys generated successfully!".green().bold());
    println!();
    println!(
        "Private key: {}",
        private_key_path.display().to_string().yellow()
    );
    println!(
        "  {} Keep this file secure! Never share it.",
        "WARNING:".red().bold()
    );
    println!();
    println!(
        "Public key:  {}",
        public_key_path.display().to_string().yellow()
    );
    println!();
    println!(
        "{}",
        "Add this to your bot's config (admin.public_key):".cyan()
    );
    println!("  {}", public_key_base64.green());
    println!();

    Ok(())
}

/// Provision secrets to bot
async fn cmd_provision(
    bot_url: String,
    admin_key_path: PathBuf,
    discord_token: String,
    hf_token: Option<String>,
) -> Result<()> {
    println!("{}", "Provisioning secrets to bot...".cyan());

    // Load admin private key
    let private_key_bytes =
        fs::read(&admin_key_path).context("Failed to read admin private key")?;
    if private_key_bytes.len() != 32 {
        anyhow::bail!("Invalid private key file - expected 32 bytes");
    }
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);
    let admin_signing_key = SigningKey::from_bytes(&key_array);
    key_array.zeroize();

    // Get bot's ephemeral public key
    let client = reqwest::Client::new();
    let pubkey_url = format!("{}/pubkey", bot_url.trim_end_matches('/'));
    println!("  Fetching bot public key from {}...", pubkey_url);

    let response = client
        .get(&pubkey_url)
        .send()
        .await
        .context("Failed to connect to bot")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Bot returned error {}: {}", status, body);
    }

    let pubkey_response: PublicKeyResponse = response
        .json()
        .await
        .context("Failed to parse bot public key response")?;

    let bot_x25519_public_bytes = BASE64
        .decode(&pubkey_response.public_key)
        .context("Invalid bot public key encoding")?;
    if bot_x25519_public_bytes.len() != 32 {
        anyhow::bail!("Invalid bot public key length");
    }
    let mut bot_key_array = [0u8; 32];
    bot_key_array.copy_from_slice(&bot_x25519_public_bytes);
    let bot_x25519_public = X25519PublicKey::from(bot_key_array);

    println!("  {}", "Bot public key received".green());

    // Generate ephemeral X25519 keypair for this session
    let admin_x25519_secret = EphemeralSecret::random_from_rng(OsRng);
    let admin_x25519_public = X25519PublicKey::from(&admin_x25519_secret);

    // Compute shared secret
    let shared_secret = admin_x25519_secret.diffie_hellman(&bot_x25519_public);

    // Create secrets payload
    let secrets = SecretsPayload {
        discord_token,
        hf_token,
        custom: HashMap::new(),
    };
    let plaintext = serde_json::to_vec(&secrets).context("Failed to serialize secrets")?;

    // Encrypt with ChaCha20-Poly1305
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
        .context("Failed to create cipher")?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| anyhow::anyhow!("Encryption failed"))?;

    println!("  {}", "Secrets encrypted".green());

    // Build message to sign: admin_x25519_public || ciphertext || nonce
    let mut message = Vec::new();
    message.extend_from_slice(admin_x25519_public.as_bytes());
    message.extend_from_slice(&ciphertext);
    message.extend_from_slice(&nonce_bytes);

    // Sign with Ed25519
    let signature = admin_signing_key.sign(&message);

    println!("  {}", "Request signed".green());

    // Build provision request
    let request = ProvisionRequest {
        admin_x25519_public: BASE64.encode(admin_x25519_public.as_bytes()),
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
        signature: BASE64.encode(signature.to_bytes()),
    };

    // Send to bot
    let provision_url = format!("{}/provision", bot_url.trim_end_matches('/'));
    println!("  Sending provision request to {}...", provision_url);

    let response = client
        .post(&provision_url)
        .json(&request)
        .send()
        .await
        .context("Failed to send provision request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Provisioning failed with status {}: {}", status, body);
    }

    let provision_response: ProvisionResponse = response
        .json()
        .await
        .context("Failed to parse provision response")?;

    if provision_response.success {
        println!();
        println!("{}", "Provisioning successful!".green().bold());
        if let Some(msg) = provision_response.message {
            println!("  {}", msg);
        }
    } else {
        anyhow::bail!("Provisioning failed: {:?}", provision_response.message);
    }

    Ok(())
}

/// Check bot status
async fn cmd_status(bot_url: String) -> Result<()> {
    println!("{}", "Checking bot status...".cyan());

    let client = reqwest::Client::new();
    let status_url = format!("{}/status", bot_url.trim_end_matches('/'));

    let response = client
        .get(&status_url)
        .send()
        .await
        .context("Failed to connect to bot")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Bot returned error {}: {}", status, body);
    }

    let status_response: StatusResponse = response
        .json()
        .await
        .context("Failed to parse status response")?;

    println!();
    match status_response.status.as_str() {
        "waiting_for_provisioning" => {
            println!("Bot status: {}", "Waiting for provisioning".yellow());
            println!("Run `linguabridge-admin provision` to configure the bot.");
        }
        "provisioned" => {
            println!("Bot status: {}", "Provisioned".green());
        }
        other => {
            println!("Bot status: {}", other);
        }
    }

    Ok(())
}

/// Display public key from private key file
async fn cmd_pubkey(admin_key_path: PathBuf) -> Result<()> {
    let private_key_bytes =
        fs::read(&admin_key_path).context("Failed to read admin private key")?;
    if private_key_bytes.len() != 32 {
        anyhow::bail!("Invalid private key file - expected 32 bytes");
    }
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&private_key_bytes);
    let signing_key = SigningKey::from_bytes(&key_array);
    key_array.zeroize();

    let verifying_key = signing_key.verifying_key();
    let public_key_base64 = BASE64.encode(verifying_key.to_bytes());

    println!("{}", "Admin Public Key (base64):".cyan());
    println!("{}", public_key_base64.green());
    println!();
    println!("Add this to your bot's config file:");
    println!("  [admin]");
    println!("  public_key = \"{}\"", public_key_base64);

    Ok(())
}
