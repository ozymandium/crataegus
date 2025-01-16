use chrono::naive::serde::ts_seconds;
use chrono::NaiveDateTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The body of the HTTP message is specified by a template. See doc/GpsLogger_Settings.md for the
/// proper value that should be set for the template, which is in JSON format. This struct is used
/// to parse the JSON template into a Rust struct.
#[derive(Deserialize, Serialize, Debug)]
pub struct Body {
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
