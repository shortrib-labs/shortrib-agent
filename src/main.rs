mod agent;
mod app_home;
mod bot;
mod calendar_blocks;
mod google_calendar;
mod oauth_store;
mod state;

use agent::Agent;
use bot::SlackBot;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::from_path(".env").ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shortrib_agent=info,slack_morphism=debug"));
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let agent = Agent::new().await?;
    SlackBot::new(agent)?.start().await?;

    Ok(())
}
