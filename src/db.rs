use chrono::NaiveDateTime;
use serde::Deserialize;

use std::path::PathBuf;

/// A database entry. Each row has this structure.
#[derive(Debug)]
pub struct Entry {
    /// Timestamp of the location data
    time: NaiveDateTime,
    /// Latitude in decimal degrees
    latitude: f64,
    /// Longitude in decimal degrees
    longitude: f64,
    /// Altitude in meters, using WGS84. Note that MSL must be set false in settings.
    altitude: f64,
    /// Estimate of the accuracy of the location in meters
    accuracy: f64,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    path: PathBuf,
    user: String,
    password: String,
}

pub struct Db {}

impl Db {
    pub async fn new(config: Config) -> Self {
        // if the database does not exist, create it
        if !config.path.exists() {
            todo!();
        }
        // now connect to the database for writing and reading
        todo!();
        Db {}
    }

    /// Record a new entry in the database
    /// # Arguments
    /// * `entry`: the entry to record
    pub async fn record(&self, entry: Entry) {
        todo!();
    }

    /// Backup the database
    pub async fn backup(&self) {
        todo!();
    }
}
