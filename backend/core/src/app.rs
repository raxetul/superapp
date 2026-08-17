use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

#[allow(unused_imports)]
use crate::{controllers, models::_entities::users, tasks, workers::downloader::DownloadWorker};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![
            Box::new(crate::initializers::metrics::MetricsInitializer),
            Box::new(crate::initializers::auth::AuthInitializer),
        ])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        // loco's native auth (`/api/auth/*`) is intentionally NOT registered:
        // Rauthy is the sole authentication authority (TR-04-002). Our own
        // Rauthy-backed auth routes live under `/api/v1/auth`.
        AppRoutes::with_default_routes()
            .add_route(controllers::auth::routes())
            .add_route(controllers::admin::routes())
            .add_route(controllers::modules::routes())
            .add_route(controllers::gateway::routes())
            .add_route(controllers::events::routes())
            .add_route(controllers::meta::routes())
            .add_route(controllers::system::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
