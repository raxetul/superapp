//! P5 (TR-05-003): registered modules.
//!
//! A module is registered by its manifest (`name`, `version`, `endpoints`,
//! `permissions`, `config_schema`, `signatures`) — stored as JSON text in
//! `manifest`. `config` holds the current runtime configuration (validated
//! against the manifest's `config_schema`). `status`/`address` track the
//! runtime lifecycle. `(name, version)` is unique — a duplicate registration
//! is rejected.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Modules {
    Table,
    Name,
    Version,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "modules",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::Uuid),
                ("name", ColType::String),
                ("version", ColType::String),
                // Full manifest JSON (immutable code/contract + signatures).
                ("manifest", ColType::Text),
                // Current runtime config JSON (validated vs config_schema).
                ("config", ColType::TextNull),
                // registered | starting | ready | stopped | failed
                ("status", ColType::String),
                // Runtime address once started (host:port), else null.
                ("address", ColType::StringNull),
            ],
            &[],
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_modules_name_version")
                .table(Modules::Table)
                .col(Modules::Name)
                .col(Modules::Version)
                .unique()
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "modules").await
    }
}
