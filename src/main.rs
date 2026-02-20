use synapse_core::{config, db, create_app, AppState, stellar::HorizonClient, services::SettlementService};
use sqlx::migrate::Migrator;
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::from_env()?;

    // Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database pool
    let pool = db::create_pool(&config).await?;

    // Run migrations
    let migrator = Migrator::new(Path::new("./migrations")).await?;
    migrator.run(&pool).await?;
    tracing::info!("Database migrations completed");

    // Initialize Stellar Horizon client
    let horizon_client = HorizonClient::new(config.stellar_horizon_url.clone());
    tracing::info!("Stellar Horizon client initialized with URL: {}", config.stellar_horizon_url);

    // Initialize Settlement Service
    let _settlement_service = SettlementService::new(pool.clone());
    
    // Start background settlement worker
    let settlement_pool = pool.clone();
    tokio::spawn(async move {
        let service = SettlementService::new(settlement_pool);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Default to hourly
        loop {
            interval.tick().await;
            tracing::info!("Running scheduled settlement job...");
            match service.run_settlements().await {
                Ok(results) => {
                    if !results.is_empty() {
                        tracing::info!("Successfully generated {} settlements", results.len());
                    }
                }
                Err(e) => tracing::error!("Scheduled settlement job failed: {:?}", e),
            }
        }
    });

    // Build router with state
    let app_state = AppState { 
        db: pool,
        horizon_client,
    };
    let app = create_app(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
