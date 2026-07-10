//! P4 (TR-04-009): service-to-service API keys.
//!
//! Modules authenticate to the core with an `X-API-Key` header. The plaintext
//! key (`sk_<prefix>_<secret>`) is shown once at creation; only its SHA-256
//! hash is stored. `prefix` is the non-secret lookup handle. A key is revoked
//! by stamping `revoked_at` (revocation is a soft, auditable state).

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "api_keys",
            &[
                ("id", ColType::PkAuto),
                // Human-readable label (e.g. the module name).
                ("name", ColType::String),
                // Non-secret lookup handle embedded in the key.
                ("prefix", ColType::StringUniq),
                // SHA-256 hex of the full plaintext key.
                ("key_hash", ColType::String),
                // Set when revoked; NULL means active.
                ("revoked_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "api_keys").await
    }
}
