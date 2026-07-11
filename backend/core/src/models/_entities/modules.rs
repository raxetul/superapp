//! `SeaORM` Entity for registered modules (TR-05-003).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "modules")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pid: Uuid,
    pub name: String,
    pub version: String,
    /// Full manifest JSON.
    pub manifest: String,
    /// Current runtime config JSON (validated against the manifest schema).
    pub config: Option<String>,
    /// Lifecycle status: registered | starting | ready | stopped | failed.
    pub status: String,
    /// Runtime address (host:port) once started.
    pub address: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
