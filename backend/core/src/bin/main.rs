use loco_rs::cli;
use migration::Migrator;
use superapp_core::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
