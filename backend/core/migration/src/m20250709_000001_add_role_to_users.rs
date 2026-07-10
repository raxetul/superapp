//! P4 (TR-04-004 / TR-04-012): add a `role` column to `users`.
//!
//! Roles are the coarse principal grouping fed to Cedar (`admin` vs the
//! least-privilege `user`). New rows default to the least-privilege `user`;
//! the admin-bootstrap and provisioning logic sets `admin` explicitly.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Role,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .add_column(
                    ColumnDef::new(Users::Role)
                        .string()
                        .not_null()
                        .default("user"),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .drop_column(Users::Role)
                .to_owned(),
        )
        .await
    }
}
