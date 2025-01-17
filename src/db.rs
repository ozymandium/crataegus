use color_eyre::eyre::{Result, WrapErr};
use log::{debug, info};
use sea_orm::sea_query::{Table, TableCreateStatement};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, Schema,
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
    conn: DatabaseConnection,
}

impl Db {
    pub async fn new(config: Config) -> Result<Self> {
        // connecting with `c` option will create the file if it doesn't exist
        let db_url = format!("sqlite://{}?mode=rwc", config.path.display());
        let conn = Database::connect(&db_url)
            .await
            .wrap_err("Failed to connect to the database")?;
        info!("Database does not exist, creating it");
        let schema = Schema::new(conn.get_database_backend());
        // add all the tables
        conn.execute(
            conn.get_database_backend().build(
                schema
                    .create_table_from_entity(user::Entity)
                    .if_not_exists(),
            ),
        )
        .await
        .wrap_err("Failed to create the users table")?;
        conn.execute(
            conn.get_database_backend().build(
                schema
                    .create_table_from_entity(location::Entity)
                    .if_not_exists(),
            ),
        )
        .await
        .wrap_err("Failed to create the locations table")?;
        Ok(Db { conn })
    }

    /// Record a new location in the database
    pub async fn record(&self, loc: Location) -> Result<()> {
        let mut active_loc = loc.into_active_model();
        active_loc
            .insert(&self.conn)
            .await
            .wrap_err("Failed to insert location into database")?;

        Ok(())
    }

    pub async fn add_user(&self, username: &String, password: &String) -> Result<()> {
        let user = user::Model {
            username: username.clone(),
            password: password.clone(),
        };
        let mut active_user = user.into_active_model();
        active_user
            .insert(&self.conn)
            .await
            .wrap_err("Failed to insert user into database")?;
        Ok(())
    }

    pub async fn check_user(&self, username: &String, password: &String) -> Result<bool> {
        let user = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.conn)
            .await
            .wrap_err("Failed to query user from database")?;
        match user {
            Some(user) => Ok(user.password == *password),
            None => Ok(false),
        }
    }

    /// Backup the database
    pub async fn backup(&self, path: PathBuf) -> Result<()> {
        todo!("implement VACUUM and backup");
        Ok(())
    }

    //////////////////
    // Test Helpers //
    //////////////////

    /// Count the number of locations in the database
    #[cfg(test)]
    pub async fn location_count(&self) -> usize {
        location::Entity::find()
            .all(&self.conn)
            .await
            .unwrap()
            .len()
    }
}

/////////////////////////////////////
// Schemas for the database tables //
/////////////////////////////////////

mod location {
    use chrono::NaiveDateTime;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "locations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub username: String,
        #[sea_orm(primary_key, auto_increment = false)]
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
        #[sea_orm(primary_key, auto_increment = false)]
        pub username: String,
        pub password: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

////////////////
// Unit Tests //
////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Creates an ephemeral database for testing and tries to add identical entries to the
    /// database. The second entry should fail due to the unique constraint on the primary key.
    #[tokio::test]
    async fn test_unique_constraint() {
        let db_file = NamedTempFile::new().unwrap();
        // having a file that exists ensures that the schema existence is checked when determining
        // whether to create the tables
        let db = Db::new(Config {
            path: db_file.path().to_path_buf(),
        })
        .await
        .unwrap();
        assert_eq!(db.location_count().await, 0);
        let loc = Location {
            username: "test".to_string(),
            time: chrono::Utc::now().naive_utc(),
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            accuracy: 0.0,
        };
        db.record(loc.clone()).await.unwrap();
        assert_eq!(db.location_count().await, 1);
        let err = db.record(loc.clone()).await.unwrap_err();
        assert_eq!(err.to_string(), "Failed to insert location into database");
    }
}
