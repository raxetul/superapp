//! Registered-module persistence (TR-05-003).

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, DatabaseConnection};
use uuid::Uuid;

use crate::modules::manifest::Manifest;

pub use super::_entities::modules::{ActiveModel, Column, Entity, Model};

/// Lifecycle status values stored in `modules.status`.
pub mod status {
    pub const REGISTERED: &str = "registered";
    pub const STARTING: &str = "starting";
    pub const READY: &str = "ready";
    pub const STOPPED: &str = "stopped";
    pub const FAILED: &str = "failed";
}

impl Model {
    /// Register a module from its (already-validated) manifest. A duplicate
    /// `name`+`version` is rejected with [`ModelError::EntityAlreadyExists`].
    ///
    /// # Errors
    /// On duplicate or DB error.
    pub async fn register(db: &DatabaseConnection, manifest: &Manifest) -> ModelResult<Model> {
        if Entity::find()
            .filter(Column::Name.eq(manifest.name.clone()))
            .filter(Column::Version.eq(manifest.version.clone()))
            .one(db)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }
        let json = serde_json::to_string(manifest).map_err(|e| ModelError::Any(e.into()))?;
        let model = ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            name: ActiveValue::set(manifest.name.clone()),
            version: ActiveValue::set(manifest.version.clone()),
            manifest: ActiveValue::set(json),
            config: ActiveValue::set(None),
            status: ActiveValue::set(status::REGISTERED.to_string()),
            address: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(model)
    }

    /// Parse the stored manifest JSON.
    ///
    /// # Errors
    /// If the stored manifest is not valid JSON.
    pub fn parsed_manifest(&self) -> ModelResult<Manifest> {
        Manifest::from_json(&self.manifest).map_err(|e| ModelError::Any(e.into()))
    }

    /// Find a module by its public id.
    ///
    /// # Errors
    /// [`ModelError::EntityNotFound`] if absent, or a parse/DB error.
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        Entity::find()
            .filter(Column::Pid.eq(uuid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// Update the runtime status (and optionally the address).
    ///
    /// # Errors
    /// On DB error.
    pub async fn set_status(
        self,
        db: &DatabaseConnection,
        status: &str,
        address: Option<String>,
    ) -> ModelResult<Model> {
        let mut active = self.into_active_model();
        active.status = ActiveValue::set(status.to_string());
        active.address = ActiveValue::set(address);
        Ok(active.update(db).await?)
    }

    /// Persist a validated configuration document.
    ///
    /// # Errors
    /// On DB error.
    pub async fn set_config(
        self,
        db: &DatabaseConnection,
        config_json: &str,
    ) -> ModelResult<Model> {
        let mut active = self.into_active_model();
        active.config = ActiveValue::set(Some(config_json.to_string()));
        Ok(active.update(db).await?)
    }
}
