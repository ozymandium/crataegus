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
    pub async fn new(db: Config) -> Self {
        todo!();
        Db {}
    }

    pub async fn record(&self, entry: Entry) {
        todo!();
    }
}
