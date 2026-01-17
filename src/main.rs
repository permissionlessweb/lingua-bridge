use linguabridge::{
    admin::{self, AdminState, SharedSecretStore},
    bot, config::AppConfig, db, translation::TranslationClient, web,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging first
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linguabridge=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting LinguaBridge v{}", env!("CARGO_PKG_VERSION"));

    // Load non-sensitive configuration
    let config = AppConfig::init()?;
    info!("Configuration loaded");

    // Validate admin public key is configured
    if config.admin.public_key.is_empty() {
        error!("Admin public key not configured!");
        error!("Generate keys with: linguabridge-admin keygen");
        error!("Then set admin.public_key in config/default.toml or LINGUABRIDGE_ADMIN__PUBLIC_KEY env var");
        return Err(anyhow::anyhow!("Admin public key not configured"));
    }

    // Create secret store (initially empty)
    let secret_store = admin::create_secret_store();

    // Create admin state for provisioning
    let admin_state = Arc::new(
        AdminState::new(&config.admin.public_key, secret_store.clone())
            .map_err(|e| anyhow::anyhow!("Failed to initialize admin transport: {}", e))?,
    );

    // Start admin provisioning server
    let admin_addr = format!("{}:{}", config.admin.host, config.admin.port);
    let admin_listener = TcpListener::bind(&admin_addr).await?;
    info!("Admin provisioning server listening on http://{}", admin_addr);
    info!("Waiting for admin to provision secrets...");
    info!("Use: linguabridge-admin provision --bot-url http://{} --discord-token YOUR_TOKEN", admin_addr);

    let admin_router = admin::admin_router(admin_state.clone());
    let admin_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(admin_listener, admin_router).await {
            error!("Admin server error: {}", e);
        }
    });

    // Wait for secrets to be provisioned
    secret_store.wait_for_provisioning().await;
    info!("Secrets provisioned! Starting main application...");

    // Now we can proceed with the rest of the startup
    run_main_application(config, secret_store).await?;

    // Shutdown admin server
    admin_handle.abort();

    Ok(())
}

/// Run the main application after secrets are provisioned.
async fn run_main_application(
    config: &'static AppConfig,
    secret_store: SharedSecretStore,
) -> anyhow::Result<()> {
    // Initialize database
    let pool = SqlitePoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;
    info!("Database connected: {}", config.database.url);

    // Run migrations
    db::init_db(&pool).await?;

    // Create translation client
    let translator = Arc::new(TranslationClient::new(config));
    info!("Translation client initialized");

    // Check inference service health
    match translator.health_check().await {
        Ok(health) => {
            info!(
                "Inference service healthy: model={}, loaded={}",
                health.model, health.model_loaded
            );
        }
        Err(e) => {
            warn!(
                "Inference service not available: {}. \
                Translation will fail until the service is started.",
                e
            );
        }
    }

    // Create broadcast manager for real-time updates
    let broadcast = Arc::new(web::BroadcastManager::new());

    // Create web server state
    let web_state = web::AppState {
        pool: pool.clone(),
        broadcast: broadcast.clone(),
    };

    // Create web router
    let app = web::create_router(web_state, translator.clone());

    // Start web server in background
    let web_addr = format!("{}:{}", config.web.host, config.web.port);
    let listener = TcpListener::bind(&web_addr).await?;
    info!("Web server listening on http://{}", web_addr);

    let web_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Web server error: {}", e);
        }
    });

    // Get Discord token from secret store
    let discord_token = secret_store
        .discord_token()
        .await
        .ok_or_else(|| anyhow::anyhow!("Discord token not found in secret store"))?;

    // Start Discord bot
    info!("Starting Discord bot...");
    let bot_result = bot::start_bot_with_token(
        pool.clone(),
        translator,
        broadcast,
        &discord_token,
    )
    .await;

    // Handle bot shutdown
    match bot_result {
        Ok(()) => info!("Discord bot shut down gracefully"),
        Err(e) => {
            error!("Discord bot error: {}", e);
            return Err(anyhow::anyhow!("{}", e));
        }
    }

    // Wait for web server to finish
    web_handle.abort();

    Ok(())
}
