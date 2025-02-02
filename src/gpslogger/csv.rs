use chrono::{DateTime, FixedOffset, Utc};
use color_eyre::eyre::{eyre, Result};
use serde::Deserialize;

use std::{fs::File, path::PathBuf};

use crate::gpslogger::deserializers::{
    deserialize_date_time_fixed_offset_from_str, deserialize_date_time_utc_from_sec,
    deserialize_date_time_utc_from_str, deserialize_option_f64, deserialize_option_string,
    deserialize_option_u32,
};
use crate::schema::{Location, LocationGen, Source};

/// # CSV
/// A CSV excerpt is below:
/// ```csv
/// time,lat,lon,elevation,accuracy,bearing,speed,satellites,provider,hdop,vdop,pdop,geoidheight,ageofdgpsdata,dgpsid,activity,battery,annotation,timestamp_ms,time_offset,distance,starttimestamp_ms,profile_name,battery_charging
/// 2025-01-24T07:02:29.168Z,24.240779519081116,-11.84485614299774,1476.0,48.0,,0.0,0,gps,,,,,,,,64,,1737702149168,2025-01-24T00:02:29.168-07:00,14780.376051140634,1737686054899,Default Profile,false
/// 2025-01-24T07:23:55.551Z,24.241143584251404,-11.84490287303925,1411.0,48.0,,0.0,0,gps,,,,,,,,63,,1737703435551,2025-01-24T00:23:55.551-07:00,14821.04923758446,1737686054899,Default Profile,false
/// 2025-01-24T07:30:20.375Z,24.241090416908264,-11.84478521347046,1355.0,48.0,,0.0,0,gps,,,,,,,,62,,1737703820375,2025-01-24T00:30:20.375-07:00,14832.590979680575,1737686054899,Default Profile,false
/// 2025-01-24T07:36:54.148Z,24.24091112613678,-11.8446295261383,1414.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704214148,2025-01-24T00:36:54.148-07:00,14856.455069256903,1737686054899,Default Profile,false
/// 2025-01-24T07:40:35.889Z,24.2408287525177,-11.84476947784424,1472.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704435889,2025-01-24T00:40:35.889-07:00,14871.385559872322,1737686054899,Default Profile,false
/// 2025-01-25T07:34:09.909Z,24.7410617163024,-11.84486579207021,1378.333910142936,7.7476687,,,0,gps,,,,,,,,60,,1737790449909,2025-01-25T00:34:09.909-07:00,7081.54436921358,1737783655597,Default Profile,false
/// ```
/// Mapping of Location fields to CSV columns:
/// - time_utc: time
/// - time_local: time_offset
/// - latitude: lat
/// - longitude: lon
/// - altitude: elevation
/// - accuracy: accuracy    
///
#[derive(Deserialize, Debug)]
struct Payload {
    /// Time in ISO 8601 format.
    /// Example: 2025-01-24T07:02:29.168Z
    #[serde(deserialize_with = "deserialize_date_time_utc_from_str")]
    time: DateTime<Utc>,
    /// Latitude in decimal degrees.
    /// Example: 40.740779519081116
    lat: f64,
    /// Longitude in decimal degrees.
    /// Example: -111.84485614299774
    lon: f64,
    /// Altitude in meters.
    /// Example: 1476.0
    elevation: f64,
    /// Accuracy in meters.
    /// Example: 48.0
    accuracy: f64,
    /// Direction of travel in degrees. Unclear whether this is north-referenced.
    /// Example: 45.0
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    bearing: Option<f64>,
    /// Speed in km/h.
    /// Example: 2.4
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    speed: Option<f64>,
    /// Number of satellites used to determine location.
    /// Example: 4
    #[allow(dead_code)]
    satellites: u32,
    /// Source of the location data. Known possible values are:
    /// - gps
    #[allow(dead_code)]
    provider: String,
    /// Horizontal dilution of precision.
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    hdop: Option<f64>,
    /// Vertical dilution of precision.
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    vdop: Option<f64>,
    /// Position dilution of precision.
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    pdop: Option<f64>,
    /// Height of geoid above WGS84 ellipsoid.
    #[serde(deserialize_with = "deserialize_option_f64")]
    #[allow(dead_code)]
    geoidheight: Option<f64>,
    /// Age of differential GPS data.
    #[serde(deserialize_with = "deserialize_option_u32")]
    #[allow(dead_code)]
    ageofdgpsdata: Option<u32>,
    /// ID of the DGPS station used in differential correction.
    #[serde(deserialize_with = "deserialize_option_u32")]
    #[allow(dead_code)]
    dgpsid: Option<u32>,
    /// Activity type.
    #[serde(deserialize_with = "deserialize_option_string")]
    #[allow(dead_code)]
    activity: Option<String>,
    /// Battery level as a percentage.
    #[allow(dead_code)]
    battery: u32,
    /// Annotation.
    #[serde(deserialize_with = "deserialize_option_string")]
    #[allow(dead_code)]
    annotation: Option<String>,
    /// Unix timestamp in milliseconds.
    /// Example: 1737702149168
    #[serde(deserialize_with = "deserialize_date_time_utc_from_sec")]
    #[allow(dead_code)]
    timestamp_ms: DateTime<Utc>,
    /// Time in ISO 8601 format with local offset.
    /// Example: 2025-01-24T00:02:29.168-07:00
    #[serde(deserialize_with = "deserialize_date_time_fixed_offset_from_str")]
    time_offset: DateTime<FixedOffset>,
    /// Distance in meters.
    #[allow(dead_code)]
    distance: f64,
    /// Unix timestamp in milliseconds.
    /// Example: 1737686054899
    #[serde(deserialize_with = "deserialize_date_time_utc_from_sec")]
    #[allow(dead_code)]
    starttimestamp_ms: DateTime<Utc>,
    /// Profile name.
    #[allow(dead_code)]
    profile_name: String,
    /// Whether the battery is charging.
    #[allow(dead_code)]
    battery_charging: bool,
}

impl LocationGen for Payload {
    fn to_location(&self, username: &str) -> Location {
        Location {
            username: username.to_string(),
            time_utc: self.time,
            time_local: self.time_offset,
            latitude: self.lat,
            longitude: self.lon,
            altitude: self.elevation,
            accuracy: Some(self.accuracy as f32),
            source: Source::GpsLogger,
        }
    }
}

/// Read a CSV file and return an iterator of `Location` structs. Does not load the entire file
/// into memory.
/// # Arguments
/// * `path` - The path to the CSV file.
/// * `username` - The username to associate with the locations.
/// # Return
/// An iterator of `Location` structs.
pub fn read_csv(path: PathBuf, username: String) -> Result<impl Iterator<Item = Result<Location>>> {
    let file = File::open(path).map_err(|e| eyre!("Failed to open CSV file: {}", e))?;
    let reader = csv::Reader::from_reader(file);
    let iter = reader
        .into_deserialize::<Payload>()
        .map(move |result| match result {
            Ok(payload) => Ok(payload.to_location(&username)),
            Err(e) => Err(eyre!("Failed to deserialize CSV row: {}", e)),
        });
    Ok(iter)
}

////////////////
// Unit Tests //
////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::NamedTempFile;

    static CSV_DATA: &str = r#"time,lat,lon,elevation,accuracy,bearing,speed,satellites,provider,hdop,vdop,pdop,geoidheight,ageofdgpsdata,dgpsid,activity,battery,annotation,timestamp_ms,time_offset,distance,starttimestamp_ms,profile_name,battery_charging
2025-01-24T07:02:29.168Z,24.240779519081116,-11.84485614299774,1476.0,48.0,,0.0,0,gps,,,,,,,,64,,1737702149168,2025-01-24T00:02:29.168-07:00,14780.376051140634,1737686054899,Default Profile,false
2025-01-24T07:23:55.551Z,24.241143584251404,-11.84490287303925,1411.0,48.0,,0.0,0,gps,,,,,,,,63,,1737703435551,2025-01-24T00:23:55.551-07:00,14821.04923758446,1737686054899,Default Profile,false
2025-01-24T07:30:20.375Z,24.241090416908264,-11.84478521347046,1355.0,48.0,,0.0,0,gps,,,,,,,,62,,1737703820375,2025-01-24T00:30:20.375-07:00,14832.590979680575,1737686054899,Default Profile,false
2025-01-24T07:36:54.148Z,24.24091112613678,-11.8446295261383,1414.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704214148,2025-01-24T00:36:54.148-07:00,14856.455069256903,1737686054899,Default Profile,false
2025-01-24T07:40:35.889Z,24.2408287525177,-11.84476947784424,1472.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704435889,2025-01-24T00:40:35.889-07:00,14871.385559872322,1737686054899,Default Profile,false
2025-01-25T07:34:09.909Z,24.7410617163024,-11.84486579207021,1378.333910142936,7.7476687,,,0,gps,,,,,,,,60,,1737790449909,2025-01-25T00:34:09.909-07:00,7081.54436921358,1737783655597,Default Profile,false"#;

    static USERNAME: &str = "test_user";

    fn create_csv() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(CSV_DATA.as_bytes()).unwrap();
        file
    }

    #[tokio::test]
    async fn test_read_csv() {
        let file = create_csv();
        let path = file.path().to_path_buf();
        let username = USERNAME.to_string();
        let iter = read_csv(path, username).unwrap();
        let locations: Vec<Result<Location>> = iter.collect();
        assert_eq!(locations.len(), 6);

        assert_eq!(
            locations[0].as_ref().unwrap().time_utc.to_rfc3339(),
            "2025-01-24T07:02:29.168+00:00"
        );
        assert_eq!(
            locations[0].as_ref().unwrap().time_local.to_rfc3339(),
            "2025-01-24T00:02:29.168-07:00"
        );
        assert_eq!(locations[0].as_ref().unwrap().latitude, 24.240779519081116);
        assert_eq!(locations[0].as_ref().unwrap().longitude, -11.84485614299774);
        assert_eq!(locations[0].as_ref().unwrap().altitude, 1476.0);
        assert_eq!(locations[0].as_ref().unwrap().accuracy, Some(48.0));
        assert_eq!(locations[0].as_ref().unwrap().source, Source::GpsLogger);
        assert_eq!(locations[0].as_ref().unwrap().username, USERNAME);

        assert_eq!(
            locations[5].as_ref().unwrap().time_utc.to_rfc3339(),
            "2025-01-25T07:34:09.909+00:00"
        );
        assert_eq!(
            locations[5].as_ref().unwrap().time_local.to_rfc3339(),
            "2025-01-25T00:34:09.909-07:00"
        );
        assert_eq!(locations[5].as_ref().unwrap().latitude, 24.7410617163024);
        assert_eq!(locations[5].as_ref().unwrap().longitude, -11.84486579207021);
        assert_eq!(locations[5].as_ref().unwrap().altitude, 1378.333910142936);
        assert_eq!(locations[5].as_ref().unwrap().accuracy, Some(7.7476687));
        assert_eq!(locations[5].as_ref().unwrap().source, Source::GpsLogger);
        assert_eq!(locations[5].as_ref().unwrap().username, USERNAME);
    }
}
