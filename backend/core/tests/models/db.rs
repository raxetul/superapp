//! SeaORM database integration — round-trip against the isolated test DB.
//!
//! Covers TR-03-002: the backend connects to PostgreSQL through a pooled
//! SeaORM connection (wired via the P2 config) and a write is durably
//! read back within an isolated test database.

use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use serial_test::serial;
use superapp_core::{app::App, models::_entities::users};

#[tokio::test]
#[serial]
async fn seaorm_insert_is_read_back() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");
    let db = &boot.app_context.db;

    // Write.
    let inserted = users::ActiveModel {
        email: ActiveValue::set("roundtrip@superapp.test".to_string()),
        name: ActiveValue::set("Round Trip".to_string()),
        password: ActiveValue::set("password-hash".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("insert should succeed against the test database");

    // Read back by primary key.
    let fetched = users::Entity::find_by_id(inserted.id)
        .one(db)
        .await
        .expect("query should succeed")
        .expect("the inserted row should be found");

    assert_eq!(fetched.email, "roundtrip@superapp.test");
    assert_eq!(fetched.name, "Round Trip");
}
