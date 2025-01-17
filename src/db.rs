use chrono::{Local, NaiveDateTime};
use color_eyre::eyre::{Result, WrapErr};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, Database, DatabaseConnection, EntityTrait,
    IntoActiveModel, Schema, ConnectionTrait,
};
use serde::Deserialize;
use log::info;

use std::path::PathBuf;

/// Use the `location::Model` as `Location` for simplicity.
pub use location::Model as Location;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    path: PathBuf,
    user: String,
    password: String,
}

pub struct Db {
    db: DatabaseConnection,
    path: PathBuf,
}

impl Db {
    pub async fn new(config: Config) -> Result<Self> {
        let db_exists = config.path.exists();
        let db_url = format!("sqlite://{}", config.path.display());
        // Connect to the database (it will create the file if it doesn't exist)
        let db = Database::connect(&db_url)
            .await
            .wrap_err("Failed to connect to the database")?;
        // If the database file didn't exist before, create the table
        if !db_exists {
            info!("Database does not exist, creating it");
            let schema = Schema::new(db.get_database_backend());
            let stmt = schema.create_table_from_entity(location::Entity);
            db.execute(db.get_database_backend().build(&stmt))
                .await
                .wrap_err("Failed to create the entries table")?;
        }
        Ok(Db {
            db,
            path: config.path.clone(),
        })
    }

    /// Record a new location in the database
    pub async fn record(&self, loc: Location) -> Result<()> {
        let mut active_loc = loc.into_active_model();
        active_loc.id = NotSet; // Unset the `id` field for insertion
        active_loc
            .insert(&self.db)
            .await
            .wrap_err("Failed to insert location into database")?;

        Ok(())
    }

    /// Backup the database
    pub async fn backup(&self) -> Result<()> {
        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let backup_path = self.path.with_extension(format!("backup_{}", timestamp));
        tokio::fs::copy(&self.path, &backup_path)
            .await
            .wrap_err("Failed to backup the database")?;

        Ok(())
    }
}

mod location {
    use chrono::NaiveDateTime;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "locations")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub time: NaiveDateTime,
        pub latitude: f64,
        pub longitude: f64,
        pub altitude: f64,
        pub accuracy: f32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
