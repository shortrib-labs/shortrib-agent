mod agent;
mod bot;
mod google_calendar;
mod state;

use agent::Agent;
use bot::SlackBot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::from_path(".env").ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("slack_morphism=debug")
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let agent = Agent::new().await?;
    SlackBot::new(agent)?.start().await?;

    Ok(())
}
