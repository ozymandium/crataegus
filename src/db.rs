use color_eyre::eyre::{eyre, Result, WrapErr};
use log::{debug, info, LevelFilter};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, Schema, SqlErr,
};
use serde::Deserialize;

use std::path::PathBuf;

use crate::schema::{location, user, Location, SanityCheck};

/// Configuration for the database, obtained from main.rs::Args
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Path to the SQLite database file
    path: PathBuf,
}

/// The database struct used by the server and the app. SQLite is used as the database backend, and
/// all storage happens through this struct.
pub struct Db {
    /// The database connection
    conn: DatabaseConnection,
}

impl Db {
    /// Create a new database connection. If the database does not exist, it will be created and
    /// the necessary tables will be added.
    /// # Arguments
    /// * `config` - The configuration for the database
    /// # Returns
    /// The database struct
    pub async fn new(config: Config) -> Result<Self> {
        // connecting with `c` option will create the file if it doesn't exist
        let url = format!("sqlite://{}?mode=rwc", config.path.display());
        let mut options = ConnectOptions::new(url);
        options.sqlx_logging_level(LevelFilter::Debug); // sqlx logging is always debug
        let conn = Database::connect(options)
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

    ////////////////////////////
    // User-Related Functions //
    ////////////////////////////

    /// Insert a new user into the database. If the user already exists, this function will return
    /// an error.
    /// # Arguments
    /// * `username` - The username to insert
    /// * `password` - The password to insert
    /// # Returns
    /// `Ok(())` if the user was successfully inserted, an error otherwise
    pub async fn user_insert(&self, username: &String, password: &String) -> Result<()> {
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

    ////////////////////////////////
    // Location-Related Functions //
    ////////////////////////////////

    /// Record a new location in the database. Silently ignore entries that are perfect duplicates,
    /// which may occur as a result of manual uploads. Duplicated user/time info with different
    /// location data will return an error.
    /// # Arguments
    /// * `loc` - The location to record
    /// # Returns
    /// `Ok(())` if the location was successfully recorded, or already exists in the database. An
    /// error otherwise.
    pub async fn location_insert(&self, loc: Location) -> Result<()> {
        loc.sanity_check()?;
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
        // add user to the database
        db.user_insert(&"test".to_string(), &"pass".to_string())
            .await
            .unwrap();
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
            accuracy: Some(0.0),
            source: location::Source::GpsLogger,
        };
        db.location_insert(loc.clone()).await.unwrap();
        assert_eq!(db.location_count().await, 1); // successfully added the first entry
        db.location_insert(loc.clone()).await.unwrap(); // adding again does nothing
        assert_eq!(db.location_count().await, 1);
        let mut loc2 = loc.clone();
        loc2.time_utc += chrono::Duration::seconds(1); // modify the time to make it unique
        assert!(db.location_insert(loc2.clone()).await.is_err()); // but the 2 times don't match
        loc2.time_local += chrono::Duration::seconds(1); // now the times are unique and match
        db.location_insert(loc2.clone()).await.unwrap();
        assert_eq!(db.location_count().await, 2); // successfully added the second entry
        let loc3 = Location {
            username: username.clone(),
            time_utc: time_utc.clone(),
            time_local: time_local.clone(),
            latitude: 1.0,
            longitude: 1.0,
            altitude: 1.0,
            accuracy: Some(1.0),
            source: location::Source::Jpeg,
        };
        let err = db.location_insert(loc3).await.unwrap_err(); // same user/time with different location
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
        db.user_insert(&"user".to_string(), &"pass".to_string())
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

    // creates an ephemeral database and checks the username relation
    #[tokio::test]
    async fn test_username_foreign_key_relation() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(Config {
            path: db_file.path().to_path_buf(),
        })
        .await
        .unwrap();
        let valid_username = "user".to_string();
        let invalid_username = "invalid".to_string();
        let mut loc = Location {
            username: valid_username.clone(),
            time_utc: DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
                .unwrap()
                .with_timezone(&Utc),
            time_local: DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
                .unwrap()
                .with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            accuracy: Some(0.0),
            source: location::Source::GpsLogger,
        };
        // insert the location should fail since no user exists
        assert!(db.location_insert(loc.clone()).await.is_err());
        // insert the user
        db.user_insert(&valid_username, &"pass".to_string())
            .await
            .unwrap();
        // insert the location should succeed now
        db.location_insert(loc.clone()).await.unwrap();
        // insert the location with an invalid username should fail
        loc.username = invalid_username.clone();
        assert!(db.location_insert(loc.clone()).await.is_err());
    }
}
