use color_eyre::eyre::{eyre, Result, WrapErr};
use log::{debug, info};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, Schema, SqlErr,
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

    /// Record a new location in the database. Silently ignore entries that are perfect duplicates,
    /// which may occur as a result of manual uploads. Duplicated user/time info with different
    /// location data will return an error.
    /// # Arguments
    /// * `loc` - The location to record
    /// # Returns
    /// `Ok(())` if the location was successfully recorded, or already exists in the database. An
    /// error otherwise.
    pub async fn record(&self, loc: Location) -> Result<()> {
        // check for NaNs
        if loc.latitude.is_nan()
            || loc.longitude.is_nan()
            || loc.altitude.is_nan()
            || loc.accuracy.is_nan()
        {
            return Err(eyre!("Location contains NaNs"));
        }

        let active_loc = loc.clone().into_active_model();
        match active_loc.insert(&self.conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if let Some(SqlErr::UniqueConstraintViolation(_)) = e.sql_err() {
                    let orig = location::Entity::find()
                        .filter(location::Column::Username.eq(loc.username.clone()))
                        .filter(location::Column::TimeUtc.eq(loc.time_utc))
                        .one(&self.conn)
                        .await
                        .wrap_err("Failed to query original location when investigating duplicate")?
                        .ok_or_else(|| eyre!("Got unique constraint violation but couldn't find the original:\n{:?}", loc))?;
                    if loc == orig {
                        debug!("Ignoring duplicate location entry: {:?}", loc);
                        Ok(())
                    } else {
                        Err(e).wrap_err(format!("Received user/time info that is duplicated, but other fields differ.\nOriginal: {:?}\nReceived: {:?}", orig, loc))
                    }
                } else {
                    Err(e).wrap_err("Failed to insert location into database")
                }
            }
        }
    }

    pub async fn user_add(&self, username: &String, password: &String) -> Result<()> {
        let user = user::Model {
            username: username.clone(),
            password: password.clone(),
        };
        let active_user = user.into_active_model();
        active_user
            .insert(&self.conn)
            .await
            .wrap_err("Failed to insert user into database")?;
        Ok(())
    }

    /// Check if the user exists in the database and if the password matches. Returns false if
    /// either the user does not exist or the user does exist, but the password does not match.
    /// # Arguments
    /// * `username` - The username to check
    /// * `password` - The password to check
    /// # Returns
    /// `Ok(true)` if the user exists and the password matches, `Ok(false)` if the user does not
    pub async fn user_check(&self, username: &String, password: &String) -> Result<bool> {
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
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "locations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub username: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub time_utc: DateTime<Utc>,
        pub time_local: DateTime<FixedOffset>,
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
    use chrono::{DateTime, Utc};
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
        let username = "test".to_string();
        let time_utc = DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
            .unwrap()
            .with_timezone(&Utc);
        let time_local = time_utc.with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap());
        let loc = Location {
            username: username.clone(),
            time_utc: time_utc.clone(),
            time_local: time_local.clone(),
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            accuracy: 0.0,
        };
        db.record(loc.clone()).await.unwrap();
        assert_eq!(db.location_count().await, 1); // successfully added the first entry
        db.record(loc.clone()).await.unwrap(); // adding again does nothing
        assert_eq!(db.location_count().await, 1);
        let mut loc2 = loc.clone();
        loc2.time_utc += chrono::Duration::seconds(1); // modify the time to make it unique
        db.record(loc2).await.unwrap();
        assert_eq!(db.location_count().await, 2); // successfully added the second entry
        let loc3 = Location {
            username: username.clone(),
            time_utc: time_utc.clone(),
            time_local: time_local.clone(),
            latitude: 1.0,
            longitude: 1.0,
            altitude: 1.0,
            accuracy: 1.0,
        };
        let err = db.record(loc3).await.unwrap_err(); // same user/time with different location
        assert!(err
            .to_string()
            .contains("Received user/time info that is duplicated, but other fields differ."));
        assert_eq!(db.location_count().await, 2); // failed to add the third entry
    }

    /// Creates an ephemeral database and checks user table operations.
    #[tokio::test]
    async fn test_user_table() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(Config {
            path: db_file.path().to_path_buf(),
        })
        .await
        .unwrap();
        assert_eq!(
            db.user_check(&"user".to_string(), &"pass".to_string())
                .await
                .unwrap(),
            false
        );
        db.user_add(&"user".to_string(), &"pass".to_string())
            .await
            .unwrap();
        assert_eq!(
            db.user_check(&"user".to_string(), &"pass".to_string())
                .await
                .unwrap(),
            true
        );
        assert_eq!(
            db.user_check(&"user".to_string(), &"wrong".to_string())
                .await
                .unwrap(),
            false
        );
    }
}
