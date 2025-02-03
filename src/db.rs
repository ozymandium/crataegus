use chrono::{DateTime, Utc};
use color_eyre::eyre::{eyre, Result, WrapErr};
//use futures::{Stream, StreamExt};
use futures::Stream;
use log::{debug, LevelFilter};
use sea_orm::{
    error::DbErr, ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder,
    Schema, SqlErr,
};
use serde::Deserialize;

use std::{
    iter::Iterator,
    path::{Path, PathBuf},
};

use crate::schema::{location, user, Location, SanityCheck};

/// Configuration for the database, obtained from main.rs::Args
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Path to the SQLite database file
    pub path: PathBuf,
    /// Keep this many most recent backups
    pub backups: usize,
}

/// Struct to hold user information
#[derive(Debug)]
pub struct UserInfo {
    /// Username of the user
    pub username: String,
    /// Number of locations for the user
    pub location_count: u64,
    /// Last time the user was seen
    pub last_seen: Option<DateTime<Utc>>,
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
    pub async fn new(config: &Config) -> Result<Self> {
        // connecting with `c` option will create the file if it doesn't exist
        let url = format!("sqlite://{}?mode=rwc", config.path.display());
        let mut options = ConnectOptions::new(url);
        options.sqlx_logging_level(LevelFilter::Debug); // sqlx logging is always debug
        let conn = Database::connect(options)
            .await
            .wrap_err("Failed to connect to the database")?;
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
        Ok(Db {
            config: config.clone(),
            conn,
        })
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
    async fn backup_to(&self, path: &Path) -> Result<()> {
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

    /// Check if the path is a backup file. Backup files are named as `db.sqlite.<ts>.bak`, where
    /// `<ts>` is the current timestamp.
    /// # Arguments
    /// * `path` - The path to check
    /// # Returns
    /// `true` if the path is a backup file, `false` otherwise
    fn is_backup(&self, path: &Path) -> bool {
        let path = path.to_str().unwrap();
        if !path.starts_with(self.config.path.to_str().unwrap()) {
            return false;
        }
        let suffix = path
            .strip_prefix(self.config.path.to_str().unwrap())
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
    pub async fn user_insert(&self, username: String, password: String) -> Result<()> {
        let user = user::Model { username, password };
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
    pub async fn user_check(&self, username: &str, password: &str) -> Result<bool> {
        let user = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.conn)
            .await
            .wrap_err("Failed to query user from database")?;
        match user {
            Some(user) => {
                user.sanity_check()?;
                Ok(user.password == *password)
            }
            None => Ok(false),
        }
    }

    /// Get a list of all usernames in the database.
    /// # Returns
    /// A vector of usernames, sorted in ascending order
    pub async fn user_vec(&self) -> Result<Vec<String>> {
        let users = user::Entity::find()
            .all(&self.conn)
            .await
            .wrap_err("Failed to query users from database")?;
        let mut users = users
            .into_iter()
            .map(|user| user.username)
            .collect::<Vec<_>>();
        users.sort();
        Ok(users)
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
    /// `Ok(true)` if the location was successfully recorded, Ok(false) if the locations already exists in the database. An
    /// error otherwise.
    pub async fn location_insert(&self, loc: Location) -> Result<bool> {
        loc.sanity_check()?;
        let active_loc = loc.clone().into_active_model();
        match active_loc.insert(&self.conn).await {
            Ok(_) => Ok(true),
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
                        Ok(false)
                    } else {
                        Err(e).wrap_err(format!("Received user/time info that is duplicated, but other fields differ.\nOriginal: {:?}\nReceived: {:?}", orig, loc))
                    }
                } else if let Some(SqlErr::ForeignKeyConstraintViolation(_)) = e.sql_err() {
                    Err(e).wrap_err(format!(
                        "User `{}` does not exist in the database. Cannot insert location.",
                        loc.username
                    ))
                } else {
                    Err(e).wrap_err(format!(
                        "Failed to insert location into database for unknown reason: {:?}",
                        loc
                    ))
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
    pub async fn location_stream(
        &self,
        username: &str,
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

    /// Get a user location closest to, but not after, the specified time.
    /// # Arguments
    /// * `username` - The username to get the location for
    /// * `time` - The time to get the location for. Time of returned location will be less than or
    /// equal to this time.
    /// # Returns
    /// The location closest to, but not after, the specified time, if it exists.
    pub async fn location_at(
        &self,
        username: &str,
        time: &DateTime<Utc>,
    ) -> Result<Option<Location>> {
        let loc = location::Entity::find()
            .filter(location::Column::Username.eq(username))
            .filter(location::Column::TimeUtc.lte(*time))
            .order_by_desc(location::Column::TimeUtc)
            .one(&self.conn)
            .await
            .wrap_err("Failed to query location from database")?;
        Ok(loc)
    }

    #[cfg(test)]
    pub(crate) async fn location_vec(
        &self,
        username: &str,
        start: DateTime<Utc>,
        stop: DateTime<Utc>,
    ) -> Result<Vec<Location>> {
        use futures::StreamExt;
        let mut stream = self.location_stream(username, start, stop).await?;
        let mut vec = Vec::new();
        while let Some(loc) = stream.next().await {
            vec.push(loc?);
        }
        Ok(vec)
    }

    /// Count the number of locations in the database. If username is provided, count only the
    /// locations for that user.
    /// # Arguments
    /// * `username` - The username to count locations for. If None, count all locations.
    /// # Returns
    /// The number of locations in the database if the query was successful, an error otherwise.
    pub async fn location_count(&self, username: Option<&str>) -> Result<u64> {
        match username {
            Some(username) => {
                // ensure user exists
                if user::Entity::find()
                    .filter(user::Column::Username.eq(username))
                    .one(&self.conn)
                    .await
                    .wrap_err(format!("Failed to query user {} from database", username))?
                    .is_none()
                {
                    return Err(eyre!("User {} does not exist in the database", username));
                }
                Ok(location::Entity::find()
                    .filter(location::Column::Username.eq(username))
                    .count(&self.conn)
                    .await
                    .wrap_err(format!("Failed to count locations for user {}", username))?)
            }
            None => Ok(location::Entity::find()
                .count(&self.conn)
                .await
                .wrap_err("Failed to count locations for all users")?),
        }
    }

    //////////////////////////
    // High Level Functions //
    //////////////////////////

    /// Get information about all users in the database.
    /// # Arguments
    /// * `username` - The username to get information for. If None, get information for all users.
    /// # Returns
    /// A vector of user information structs.
    pub async fn info(&self, username: Option<&str>) -> Result<Vec<UserInfo>> {
        let users: Vec<String> = match username {
            Some(username) => vec![username.to_string()],
            None => self
                .user_vec()
                .await
                .wrap_err("Failed to query user list from database")?,
        };
        let mut user_infos = Vec::new();
        for username in users {
            let count = self.location_count(Some(&username)).await?;
            let last_seen = location::Entity::find()
                .filter(location::Column::Username.eq(&username))
                .order_by_desc(location::Column::TimeUtc)
                .one(&self.conn)
                .await
                .wrap_err("Failed to query last seen location from database")?
                .map(|loc| loc.time_utc);
            user_infos.push(UserInfo {
                username,
                location_count: count,
                last_seen,
            });
        }
        Ok(user_infos)
    }
}

////////////////
// Unit Tests //
////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use pretty_assertions::assert_eq;
    use tempfile::NamedTempFile;

    /// Creates an ephemeral database for testing and tries to add identical entries to the
    /// database. The second entry should fail due to the unique constraint on the primary key.
    #[tokio::test]
    async fn test_unique_constraint() {
        let db_file = NamedTempFile::new().unwrap();
        // having a file that exists ensures that the schema existence is checked when determining
        // whether to create the tables
        let db = Db::new(&Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 0);
        // add user to the database
        db.user_insert("test".to_string(), "pass".to_string())
            .await
            .unwrap();
        let username = "test".to_string();
        let time_utc = DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
            .unwrap()
            .with_timezone(&Utc);
        let time_local = time_utc.with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap());
        let loc = Location {
            username: username.clone(),
            time_utc,
            time_local,
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            accuracy: Some(0.0),
            source: location::Source::GpsLogger,
        };
        db.location_insert(loc.clone()).await.unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 1); // successfully added the first entry
        db.location_insert(loc.clone()).await.unwrap(); // adding again does nothing
        assert_eq!(db.location_count(None).await.unwrap(), 1);
        let mut loc2 = loc.clone();
        loc2.time_utc += chrono::Duration::seconds(1); // modify the time to make it unique
        assert!(db.location_insert(loc2.clone()).await.is_err()); // but the 2 times don't match
        loc2.time_local += chrono::Duration::seconds(1); // now the times are unique and match
        db.location_insert(loc2.clone()).await.unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 2); // successfully added the second entry
        let loc3 = Location {
            username,
            time_utc,
            time_local,
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
        assert_eq!(db.location_count(None).await.unwrap(), 2); // failed to add the third entry
    }

    /// Creates an ephemeral database and checks user table operations.
    #[tokio::test]
    async fn test_user_table() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(&Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        assert_eq!(db.user_check("user", "pass").await.unwrap(), false);
        db.user_insert("user".to_string(), "pass".to_string())
            .await
            .unwrap();
        assert_eq!(db.user_check("user", "pass").await.unwrap(), true);
        assert_eq!(db.user_check("user", "wrong").await.unwrap(), false);
        assert_eq!(db.user_check("nonexistent", "pass").await.unwrap(), false);
        assert_eq!(db.user_vec().await.unwrap(), vec!["user"]);
        db.user_insert("another_user".to_string(), "pass2".to_string())
            .await
            .unwrap();
        assert_eq!(db.user_vec().await.unwrap(), vec!["another_user", "user"]);
    }

    // creates an ephemeral database and checks the username relation
    #[tokio::test]
    async fn test_username_foreign_key_relation() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(&Config {
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
        db.user_insert(valid_username, "pass".to_string())
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
        let db = Db::new(&Config {
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
        let db = Db::new(&Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        db.user_insert("user1".to_string(), "pass".to_string())
            .await
            .unwrap();
        db.user_insert("user2".to_string(), "pass".to_string())
            .await
            .unwrap();
        let times = [
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
            let expected_idxs = [0, 2, 4];
            let mut stream = db
                .location_stream("user1", times[0], times[6])
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
            let expected_idxs = [1, 3];
            let mut stream = db
                .location_stream("user2", times[0], times[6])
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
                .location_stream("user2", times[1], times[3])
                .await
                .unwrap();
            let loc = stream.next().await.unwrap().unwrap();
            assert_eq!(loc, locs[expected_idx]);
            assert!(stream.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn test_location_count() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(&Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 0);
        // should err for nonexistent user
        assert!(db.location_count(Some("user1")).await.is_err());
        db.user_insert("user1".to_string(), "pass".to_string())
            .await
            .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 0);
        assert_eq!(db.location_count(Some("user1")).await.unwrap(), 0);
        assert!(db.location_count(Some("user2")).await.is_err());
        db.user_insert("user2".to_string(), "pass".to_string())
            .await
            .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 0);
        db.location_insert(Location {
            username: "user1".to_string(),
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
        })
        .await
        .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 1);
        assert_eq!(db.location_count(Some("user1")).await.unwrap(), 1);
        assert_eq!(db.location_count(Some("user2")).await.unwrap(), 0);
        assert!(db.location_count(Some("user3")).await.is_err());
        db.location_insert(Location {
            username: "user2".to_string(),
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
        })
        .await
        .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 2);
        assert_eq!(db.location_count(Some("user1")).await.unwrap(), 1);
        assert_eq!(db.location_count(Some("user2")).await.unwrap(), 1);
        db.location_insert(Location {
            username: "user1".to_string(),
            time_utc: DateTime::parse_from_rfc3339("2025-01-16T03:54:52.000Z")
                .unwrap()
                .with_timezone(&Utc),
            time_local: DateTime::parse_from_rfc3339("2025-01-16T03:54:52.000Z")
                .unwrap()
                .with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
            accuracy: Some(0.0),
            source: location::Source::GpsLogger,
        })
        .await
        .unwrap();
        assert_eq!(db.location_count(None).await.unwrap(), 3);
        assert_eq!(db.location_count(Some("user1")).await.unwrap(), 2);
        assert_eq!(db.location_count(Some("user2")).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_location_at() {
        let db_file = NamedTempFile::new().unwrap();
        let db = Db::new(&Config {
            path: db_file.path().to_path_buf(),
            backups: 1,
        })
        .await
        .unwrap();
        db.user_insert("user1".to_string(), "pass".to_string())
            .await
            .unwrap();
        db.user_insert("user2".to_string(), "pass".to_string())
            .await
            .unwrap();
        for i in 1..3 {
            for j in 1..3 {
                db.location_insert(Location {
                    username: format!("user{}", i),
                    time_utc: DateTime::parse_from_rfc3339(
                        format!("2025-01-16T03:54:5{}.000Z", j).as_str(),
                    )
                    .unwrap()
                    .with_timezone(&Utc),
                    time_local: DateTime::parse_from_rfc3339(
                        format!("2025-01-16T03:54:5{}.000Z", j).as_str(),
                    )
                    .unwrap()
                    .with_timezone(&chrono::FixedOffset::west_opt(3600).unwrap()),
                    latitude: i as f64,
                    longitude: 0.0,
                    altitude: 0.0,
                    accuracy: Some(0.0),
                    source: location::Source::GpsLogger,
                })
                .await
                .unwrap();
            }
        }
        assert_eq!(
            db.location_at(
                "user1",
                &DateTime::parse_from_rfc3339("2025-01-16T03:54:50.000Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .await
            .unwrap(),
            None
        );

        let loc = db
            .location_at(
                "user1",
                &DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loc.username, "user1");
        assert_eq!(
            loc.time_utc,
            DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(loc.latitude, 1.0);
        let loc = db
            .location_at(
                "user1",
                &DateTime::parse_from_rfc3339("2025-01-16T03:54:51.500Z")
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loc.username, "user1");
        assert_eq!(
            loc.time_utc,
            DateTime::parse_from_rfc3339("2025-01-16T03:54:51.000Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(loc.latitude, 1.0);
        let loc = db
            .location_at(
                "user1",
                &DateTime::parse_from_rfc3339("2025-01-16T03:54:52.000Z")
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loc.username, "user1");
        assert_eq!(
            loc.time_utc,
            DateTime::parse_from_rfc3339("2025-01-16T03:54:52.000Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(loc.latitude, 1.0);
    }
}
