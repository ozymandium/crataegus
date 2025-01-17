use chrono::{Local, NaiveDateTime};
use color_eyre::eyre::{Result, WrapErr};
use log::info;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, IntoActiveModel, Schema, QueryFilter, ColumnTrait,
};
use serde::Deserialize;

use std::path::PathBuf;

/// Use the `location::Model` as `Location` for simplicity.
pub use location::Model as Location;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    path: PathBuf,
}

pub struct Db {
    db: DatabaseConnection,
}

impl Db {
    pub async fn new(config: Config) -> Result<Self> {
        let db_exists = config.path.exists();
        let db_url = format!("sqlite://{}?mode=rwc", config.path.display());
        // Connect to the database (it will create the file if it doesn't exist, for sqlite)
        let db = Database::connect(&db_url)
            .await
            .wrap_err("Failed to connect to the database")?;
        // If the database file didn't exist before, create the table
        if !db_exists {
            info!("Database does not exist, creating it");
            let schema = Schema::new(db.get_database_backend());
            let statement = schema.create_table_from_entity(location::Entity);
            db.execute(db.get_database_backend().build(&statement))
                .await
                .wrap_err("Failed to create the entries table")?;
        }
        Ok(Db { db })
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

    pub async fn add_user(&self, username: &String, password: &String) -> Result<()> {
        let user = user::Model {
            id: 0, // Placeholder, will be set by the database
            username: username.clone(),
            password: password.clone(),
        };
        let mut active_user = user.into_active_model();
        active_user.id = NotSet;
        active_user
            .insert(&self.db)
            .await
            .wrap_err("Failed to insert user into database")?;
        Ok(())
    }

    pub async fn check_user(&self, username: &String, password: &String) -> Result<bool> {
        let user = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await
            .wrap_err(format!("Failed to find user in database: {}", username))?;
        //Ok(user.map_or(false, |u| u.password == password))
        // compare string to string reference
        Ok(user.map_or(false, |u| u.password == *password))
    }

    /// Backup the database
    pub async fn backup(&self, path: PathBuf) -> Result<()> {
        todo!("implement VACUUM and backup");
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
        pub username: String,
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

mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub username: String,
        pub password: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
