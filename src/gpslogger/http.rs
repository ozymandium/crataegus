use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde::Deserialize;

use crate::gpslogger::deserializers::{
    deserialize_date_from_str, deserialize_date_time_fixed_offset_from_str,
    deserialize_date_time_utc_from_sec, deserialize_date_time_utc_from_str, deserialize_option_f32,
};
use crate::schema::{Location, LocationGen, Source};

/// # HTTP
/// The body of the HTTP message is specified by a template that is configured in the GpsLogger app.
/// Its value should be set to `%ALL`. This will result in a body string that looks like this:
/// ```txt
/// lat=41.74108695983887&lon=-91.84490871429443&sat=0&desc=&alt=1387.0&acc=6.0&dir=170.8125&prov=gps&spd_kph=0.0&spd=0.0&timestamp=1736999691&timeoffset=2025-01-15T20:54:51.000-07:00&time=2025-01-16T03:54:51.000Z&starttimestamp=1737000139&date=2025-01-16&batt=27.0&ischarging=false&aid=4ca9e1da592aca9b&ser=4ca9e1da592aca9b&act=&filename=20250115&profile=Default+Profile&hdop=&vdop=&pdop=&dist=0&
/// ```
///
/// Breaking that up with the `&` token, it is a bit easier to visualize the key-value pairs:
/// ```txt
/// lat=41.74108695983887
/// lon=-91.84490871429443
/// sat=0
/// desc=
/// alt=1387.0
/// acc=6.0
/// dir=170.8125
/// prov=gps
/// spd_kph=0.0
/// spd=0.0
/// timestamp=1736999691
/// timeoffset=2025-01-15T20:54:51.000-07:00
/// time=2025-01-16T03:54:51.000Z
/// starttimestamp=1737000139
/// date=2025-01-16
/// batt=27.0
/// ischarging=false
/// aid=4ca9e1da592aca9b
/// ser=4ca9e1da592aca9b
/// act=
/// filename=20250115
/// profile=Default+Profile
/// hdop=
/// vdop=
/// pdop=
/// dist=0
/// ```
///
/// It can could be JSON formatted to ensure that the body string is trivially deserializeable into
/// this struct, however that would require more user configuration, and any breaking app changes
/// would require user intervention. The app-side body template field would be:
/// ```json
/// {
///   "lat": %LAT,
///   "lon": %LON,
///   "sat": %SAT,
///   "desc": "%DESC",
///   "alt": %ALT,
///   "acc": %ACC,
///   "dir": %DIR,
///   "prov": "%PROV",
///   "spd_kph": %SPD_KPH,
///   "spd": %SPD
///   "timestamp": %TIMESTAMP,
///   "timeoffset": "%TIMEOFFSET",
///   "time": "%TIME",
///   "starttimestamp": %STARTTIMESTAMP,
///   "date": "%DATE",
///   "batt": %BATT,
///   "ischarging": %ISCHARGING,
///   "aid": %AID,
///   "ser": %SER,
///   "act": "%ACT",
///   "filename": "%FILENAME",
///   "profile": "%PROFILE",
///   "hdop": %HDOP,
///   "vdop": %VDOP,
///   "pdop": %PDOP,
///   "dist": %DIST,
/// }
/// ```
///
/// Note the following fields appear with `%ALL` but do not have specific named parameters:
/// - `act`: unknown. this is ommitted from the URLPayload struct.
///
/// However, we choose to manually parse the default body string. This way, the user only needs to
/// set the template to `%ALL` and allow user app updates to be handled in the server. This struct
/// does no type conversion (e.g., for timestamps), and only stores data in the type in which it is
/// received.
///
/// Parameters that are sent in the URL of an HTTP POST request from the GpsLogger app when
/// GPSLogger is configured with the %ALL parameter. Note that this information is different from
/// the information that is available in the CSV logs.
#[derive(Deserialize, Debug)]
pub struct Payload {
    /// Latitude in decimal degrees.
    /// Example: `41.74108695983887`.
    pub lat: f64,
    /// Longitude in decimal degrees.
    /// Example: `-91.84490871429443`.
    pub lon: f64,
    /// Number of satellites in use/visible (unclear).
    /// Example: `0`.
    #[allow(dead_code)]
    sat: u8,
    /// Description of the data collection event to which this data belongs.
    /// Example: `""`, `"Hiking"`.
    #[allow(dead_code)]
    desc: String,
    /// Altitude in meters, using WGS84. Note that MSL must be set false in settings.
    /// Example: `1387.0`.
    pub alt: f64,
    /// Estimate of the accuracy of the location in meters. This is presumed to be the horizontal
    /// accuracy (earth-tangent plane).
    /// Example: `6.0`.
    pub acc: f32,
    /// Direction in degrees (unknown whether 0 is north or east, presumed north). This is also
    /// presumably direction of travel (angle of velocity vector), but may be the fused estimate of
    /// phone orientation.
    /// Example: `170.8125`.
    #[allow(dead_code)]
    dir: f32,
    /// Provider of the location data. Known possible values are:
    /// - `"gps"`: GPS location data
    #[allow(dead_code)]
    prov: String,
    /// Speed in kilometers per hour.
    /// Example: `0.0`.
    #[allow(dead_code)]
    spd_kph: f32,
    /// Speed in (meters per second?).
    /// Example: `0.0`.
    #[allow(dead_code)]
    spd: f32,
    /// Unix timestamp of the data, second-precision.
    /// Example: `1736999691`.
    #[serde(deserialize_with = "deserialize_date_time_utc_from_sec")]
    #[allow(dead_code)]
    timestamp: DateTime<Utc>,
    /// Time as an ISO 8601 string with offset.
    /// Example: `2025-01-15T20:54:51.000-07:00`.
    #[serde(deserialize_with = "deserialize_date_time_fixed_offset_from_str")]
    pub timeoffset: DateTime<FixedOffset>,
    /// Time as an ISO 8601 string in UTC. It should be the same as `timestamp`.
    /// Example: `2025-01-16T03:54:51.000Z`.
    #[serde(deserialize_with = "deserialize_date_time_utc_from_str")]
    pub time: DateTime<Utc>,
    /// Unix timestamp of the start of the data collection event, second-precision.
    /// Example: `1737000139`.
    #[serde(deserialize_with = "deserialize_date_time_utc_from_sec")]
    #[allow(dead_code)]
    starttimestamp: DateTime<Utc>,
    /// Date as an ISO 8601 string.
    /// Example: `2025-01-16`.
    #[serde(deserialize_with = "deserialize_date_from_str")]
    #[allow(dead_code)]
    date: NaiveDate,
    /// Battery percentage.
    /// Example: `27.0`.
    #[allow(dead_code)]
    batt: f32,
    /// Whether the device is charging.
    /// Example: `false`.
    #[allow(dead_code)]
    ischarging: bool,
    /// Android ID
    /// Example: `4ca9e1da592aca9b`.
    #[allow(dead_code)]
    aid: String,
    /// Serial number
    /// Example: `4ca9e1da592aca9b`.
    #[allow(dead_code)]
    ser: String,
    /// File name of the data collection event on the phone.
    /// Example: `20250115`.
    #[allow(dead_code)]
    filename: String,
    /// Profile name of the data collection event on the phone.
    /// Example: `Default Profile`.
    #[allow(dead_code)]
    profile: String,
    /// Horizontal dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    #[allow(dead_code)]
    hdop: Option<f32>,
    /// Vertical dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    #[allow(dead_code)]
    vdop: Option<f32>,
    /// Position dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    #[allow(dead_code)]
    pdop: Option<f32>,
    /// Distance traveled. Unclear whether this is distance from last data point, distance from
    /// last sent point, or distance since start of data collection event.
    /// Example: `0`.
    #[allow(dead_code)]
    dist: f32,
}

impl LocationGen for Payload {
    /// Convert the Payload struct to a Location struct.
    /// # Arguments
    /// * `username` - The username to associate with the location.
    /// # Return
    /// A Location struct with the data from the Payload struct.
    fn to_location(&self, username: &String) -> Location {
        Location {
            username: username.clone(),
            time_utc: self.time,
            time_local: self.timeoffset,
            latitude: self.lat,
            longitude: self.lon,
            altitude: self.alt,
            accuracy: Some(self.acc),
            source: Source::GpsLogger,
        }
    }
}

////////////////
// Unit Tests //
////////////////

#[cfg(test)]
mod tests {
    use super::*;

    /// Define a common HTTP body string for testing.
    const BODY_STR: &str = "lat=41.74108695983887&lon=-91.84490871429443&sat=0&desc=&alt=1387.0&acc=6.0&dir=170.8125&prov=gps&spd_kph=0.0&spd=0.0&timestamp=1736999691&timeoffset=2025-01-15T20:54:51.000-07:00&time=2025-01-16T03:54:51.000Z&starttimestamp=1737000139&date=2025-01-16&batt=27.0&ischarging=false&aid=4ca9e1da592aca9b&ser=4ca9e1da592aca9b&act=&filename=20250115&profile=Default+Profile&hdop=&vdop=&pdop=&dist=0";

    /// An actual body string observed from the GpsLogger app.
    #[test]
    fn test_from_http_body() {
        let payload: Payload = serde_urlencoded::from_str(BODY_STR).unwrap();
        assert_eq!(payload.lat, 41.74108695983887);
        assert_eq!(payload.lon, -91.84490871429443);
        assert_eq!(payload.sat, 0);
        assert_eq!(payload.desc, "");
        assert_eq!(payload.alt, 1387.0);
        assert_eq!(payload.acc, 6.0);
        assert_eq!(payload.dir, 170.8125);
        assert_eq!(payload.prov, "gps");
        assert_eq!(payload.spd_kph, 0.0);
        assert_eq!(payload.spd, 0.0);
        assert_eq!(payload.timestamp.timestamp(), 1736999691);
        assert_eq!(payload.timeoffset.to_rfc3339(), "2025-01-15T20:54:51-07:00");
        assert_eq!(payload.time.to_rfc3339(), "2025-01-16T03:54:51+00:00");
        assert_eq!(payload.starttimestamp.timestamp(), 1737000139);
        assert_eq!(payload.date.to_string(), "2025-01-16");
        assert_eq!(payload.batt, 27.0);
        assert_eq!(payload.ischarging, false);
        assert_eq!(payload.aid, "4ca9e1da592aca9b");
        assert_eq!(payload.ser, "4ca9e1da592aca9b");
        assert_eq!(payload.filename, "20250115");
        assert_eq!(payload.profile, "Default Profile");
        assert_eq!(payload.hdop, None);
        assert_eq!(payload.vdop, None);
        assert_eq!(payload.pdop, None);
        assert_eq!(payload.dist, 0.0);

        // several repeated timestamps should all be the same
        assert_eq!(payload.timestamp, payload.time);
        assert_eq!(payload.timeoffset, payload.time);
    }

    /// Test the conversion of a Payload struct to a Location struct.
    #[test]
    fn test_to_location() {
        let payload: Payload = serde_urlencoded::from_str(BODY_STR).unwrap();
        let username = "testuser".to_string();
        let location = LocationGen::to_location(&payload, &username);
        assert_eq!(location.username, username);
        assert_eq!(location.time_utc, payload.time);
        assert_eq!(location.time_local, payload.timeoffset);
        assert_eq!(location.latitude, payload.lat);
        assert_eq!(location.longitude, payload.lon);
        assert_eq!(location.altitude, payload.alt);
        assert_eq!(location.accuracy, Some(payload.acc));
        assert_eq!(location.source, Source::GpsLogger);
    }
}
