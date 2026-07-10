//! P4 (TR-04-013): admin email allow-list.
//!
//! When self-registration is disabled (the default), only identities whose
//! **email** an admin has pre-authorized may authenticate via any Rauthy
//! method. This table is that allow-list; email is the identity key.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "allowlisted_emails",
            &[
                ("id", ColType::PkAuto),
                ("email", ColType::StringUniq),
                // pid (email) of the admin who added the entry; null for seeds.
                ("added_by", ColType::StringNull),
            ],
            &[],
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "allowlisted_emails").await
    }
}
