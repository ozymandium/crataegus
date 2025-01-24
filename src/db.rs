use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result, WrapErr};
use futures::Stream;
use log::{debug, info, LevelFilter};
use sea_orm::{
    error::DbErr, ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Schema, SqlErr,
};
use serde::Deserialize;

use std::{iter::Iterator, path::PathBuf};

use crate::schema::{location, user, Location, SanityCheck};

/// Configuration for the database, obtained from main.rs::Args
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Path to the SQLite database file
    path: PathBuf,
    /// Keep this many most recent backups
    backups: usize,
}

/// The database struct used by the server and the app. SQLite is used as the database backend, and
/// all storage happens through this struct.
pub struct Db {
    /// Configuration
    config: Config,
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
        Ok(Db { config, conn })
    }

    //////////////////////
    // Backup Functions //
    //////////////////////

    /// Create a backup of the database at the specified path.
    /// # Arguments
    /// * `path` - The path where the backup should be created.
    ///     - Must be an absolute path.
    ///     - The path must not exist as any fs object.
    ///     - The parent directory must exist.
    ///     - Parent must not be root.
    /// # Returns
    /// `Ok(())` if the backup was successfully created, an error otherwise
    async fn backup_to(&self, path: &PathBuf) -> Result<()> {
        // check that the path is absolute
        if !path.is_absolute() {
            return Err(eyre!("Backup path must be an absolute path: {:?}", path));
        }
        // check that the backup path does not exist
        match path.try_exists() {
            Ok(true) => return Err(eyre!("Backup path already exists: {:?}", path)),
            Err(e) => return Err(eyre!("Failed to check if backup path exists: {:?}", e)),
            _ => (),
        }
        // check that the parent directory already exists, do not create it.
        if let Some(parent) = path.parent() {
            match parent.try_exists() {
                Ok(false) => return Err(eyre!("Parent directory does not exist: {:?}", parent)),
                Err(e) => return Err(eyre!("Failed to check if parent directory exists: {:?}", e)),
                _ => (),
            }
        } else {
            return Err(eyre!(
                "Cannot fetch parent directory of backup path: {:?}",
                path
            ));
        }

        // Ensure the path is correctly escaped to prevent SQL injection
        let cmd = format!("VACUUM INTO '{}'", path.display());
        self.conn
            .execute(sea_orm::Statement::from_string(
                self.conn.get_database_backend(),
                cmd,
            ))
            .await
            .wrap_err("Failed to create database backup")?;
        Ok(())
    }

    fn is_backup(&self, path_buf: &PathBuf) -> bool {
        let path = path_buf.to_str().unwrap();
        if !path.starts_with(&self.config.path.to_str().unwrap()) {
            return false;
        }
        let suffix = path
            .strip_prefix(&self.config.path.to_str().unwrap())
            .unwrap();
        let parts = suffix.split('.').collect::<Vec<_>>();
        parts.len() == 3 && parts[1].parse::<i64>().is_ok() && parts[2] == "bak"
    }

    /// Backups live in the same directory as the database. A db with path `/path/to/db.sqlite` will
    /// have a backup at `/path/to/db.sqlite.<ts>.bak`, where `<ts>` is the current timestamp.
    /// After backup creation, a maximum of
    pub async fn backup(&self) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        // create the backup
        let backup_path = PathBuf::from(format!("{}.{}.bak", self.config.path.display(), now));
        debug!("Creating backup at: {:?}", backup_path);
        self.backup_to(&backup_path).await?;
        // delete any old backups until `config.backups` backups remain in the directory.
        let dir = backup_path.parent().unwrap();
        let mut backups = dir
            .read_dir()
            .wrap_err("Failed to read backup directory")?
            .map(|entry| entry.unwrap().path())
            .filter(|path| self.is_backup(path))
            .collect::<Vec<_>>();
        backups.sort();
        backups.reverse();
        while backups.len() > self.config.backups {
            let to_delete = backups.pop().unwrap();
            debug!("Deleting old backup: {:?}", to_delete);
            std::fs::remove_file(&to_delete)
                .wrap_err(format!("Failed to delete backup: {:?}", to_delete))?;
        }
        Ok(())
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

    /// Generator function that returns all locations in the database that fall between the
    /// specified time bounds. Avoids loading all locations into memory at once. Lifetime is tied
    /// to the database connection.
    /// # Arguments
    /// * `start` - The start time of the range, inclusive.
    /// * `stop` - The stop time of the range, exclusive.
    /// # Returns
    /// Locations that fall within the specified time range, in ascending order of time.
    pub async fn location_get(
        &self,
        username: &String,
        start: DateTime<Utc>,
        stop: DateTime<Utc>,
    ) -> Result<impl Stream<Item = Result<Location, DbErr>> + use<'_>, DbErr> {
        let stream = location::Entity::find()
            .filter(location::Column::Username.eq(username))
            .filter(location::Column::TimeUtc.between(start, stop))
            .order_by_asc(location::Column::TimeUtc)
            .stream(&self.conn)
            .await?;
        Ok(stream)
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
            backups: 1,
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
            source: location::Source::GpsLogger,
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
            backups: 1,
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
            backups: 1,
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

    #[tokio::test]
    async fn test_is_backup() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(Config {
            path: db_file.path().to_path_buf(),
            backups: 3,
        })
        .await
        .unwrap();
        let paths_and_expectations = vec![
            (
                db_file.path().to_path_buf().with_extension("123456789.bak"),
                true,
            ),
            (
                db_file.path().to_path_buf().with_extension("123456789"),
                false,
            ),
            (
                db_file
                    .path()
                    .to_path_buf()
                    .with_extension("123456789.bak2"),
                false,
            ),
            (
                db_file
                    .path()
                    .to_path_buf()
                    .with_extension("123456789.bak."),
                false,
            ),
            (db_file.path().to_path_buf().with_extension("1.bak"), true),
            (PathBuf::from("/tmp/123456789.bak"), false),
            (PathBuf::from("/tmp/db.sqlite.123456789.bak"), false),
        ];
        println!("{:?}", db_file.path());
        for (path, expected) in paths_and_expectations {
            println!("{:?} -> {}", path, expected);
            assert_eq!(db.is_backup(&path), expected);
        }
    }

    #[tokio::test]
    async fn test_location_get() {
        use futures::StreamExt;
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        db.user_insert(&"user1".to_string(), &"pass".to_string())
            .await
            .unwrap();
        db.user_insert(&"user2".to_string(), &"pass".to_string())
            .await
            .unwrap();
        let times = vec![
            DateTime::parse_from_rfc3339("2024-12-31T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-01T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-02T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-03T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-04T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-05T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339("2025-01-06T00:00:00.000Z")
                .unwrap()
                .with_timezone(&Utc),
        ];
        let locs = vec![
            Location {
                username: "user1".to_string(),
                time_utc: times[1],
                time_local: times[1].with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                latitude: 1.0,
                longitude: 1.0,
                altitude: 1.0,
                accuracy: Some(1.0),
                source: location::Source::GpsLogger,
            },
            Location {
                username: "user2".to_string(),
                time_utc: times[2],
                time_local: times[2].with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                latitude: 2.0,
                longitude: 2.0,
                altitude: 2.0,
                accuracy: Some(2.0),
                source: location::Source::GpsLogger,
            },
            Location {
                username: "user1".to_string(),
                time_utc: times[3],
                time_local: times[3].with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                latitude: 3.0,
                longitude: 3.0,
                altitude: 3.0,
                accuracy: Some(3.0),
                source: location::Source::GpsLogger,
            },
            Location {
                username: "user2".to_string(),
                time_utc: times[4],
                time_local: times[4].with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                latitude: 4.0,
                longitude: 4.0,
                altitude: 4.0,
                accuracy: Some(4.0),
                source: location::Source::GpsLogger,
            },
            Location {
                username: "user1".to_string(),
                time_utc: times[5],
                time_local: times[5].with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                latitude: 5.0,
                longitude: 5.0,
                altitude: 5.0,
                accuracy: Some(5.0),
                source: location::Source::GpsLogger,
            },
        ];
        for loc in locs.iter() {
            db.location_insert(loc.clone()).await.unwrap();
        }
        // first just do an easy one with a bound that includes all the locations and check that
        // the user filter works
        {
            let expected_idxs = vec![0, 2, 4];
            let mut stream = db
                .location_get(&"user1".to_string(), times[0], times[6])
                .await
                .unwrap();
            let mut count = 0;
            while let Some(loc) = stream.next().await {
                assert!(count < expected_idxs.len());
                let loc = loc.unwrap();
                assert_eq!(loc, locs[expected_idxs[count]]);
                count += 1;
            }
            assert_eq!(count, expected_idxs.len());
        }
        {
            let expected_idxs = vec![1, 3];
            let mut stream = db
                .location_get(&"user2".to_string(), times[0], times[6])
                .await
                .unwrap();
            let mut count = 0;
            while let Some(loc) = stream.next().await {
                assert!(count < expected_idxs.len());
                let loc = loc.unwrap();
                assert_eq!(loc, locs[expected_idxs[count]]);
                count += 1;
            }
            assert_eq!(count, expected_idxs.len());
        }
        // now grab only a subset
        {
            let expected_idx = 1;
            let mut stream = db
                .location_get(&"user2".to_string(), times[1], times[3])
                .await
                .unwrap();
            let loc = stream.next().await.unwrap().unwrap();
            assert_eq!(loc, locs[expected_idx]);
            assert!(stream.next().await.is_none());
        }
    }
}
