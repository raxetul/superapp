#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20250709_000001_add_role_to_users;
mod m20250709_000002_allowlisted_emails;
mod m20250709_000003_api_keys;
mod m20250711_000001_modules;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20250709_000001_add_role_to_users::Migration),
            Box::new(m20250709_000002_allowlisted_emails::Migration),
            Box::new(m20250709_000003_api_keys::Migration),
            Box::new(m20250711_000001_modules::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
