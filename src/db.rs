use chrono::naive::serde::ts_seconds;
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A database entry
#[derive(Deserialize, Serialize, Debug)]
pub struct Entry {
    /// Timestamp of the location data
    #[serde(with = "ts_seconds")]
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
