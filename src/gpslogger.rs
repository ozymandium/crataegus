use chrono::naive::serde::ts_seconds;
use chrono::NaiveDateTime;
use color_eyre::eyre::{eyre, Result};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Some fields are optional floats that may be empty. Give serde a way to deserialize those.
///
/// # Arguments
/// * `deserializer` - The serde deserializer.
///
/// # Return
/// An Option<f32> if the field is present, or None if it is not.
fn deserialize_option_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("") | None => Ok(None),
        Some(s) => s.parse::<f32>().map(Some).map_err(de::Error::custom),
    }
}

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
/// - `act`: unknown
///
/// However, we choose to manually parse the default body string. This way, the user only needs to
/// set the template to `%ALL` and allow user app updates to be handled in the server. This struct
/// does no type conversion (e.g., for timestamps), and only stores data in the type in which it is
/// received.
#[derive(Deserialize, Serialize, Debug)]
pub struct Payload {
    /// Latitude in decimal degrees.
    /// Example: `41.74108695983887`.
    lat: f64,
    /// Longitude in decimal degrees.
    /// Example: `-91.84490871429443`.
    lon: f64,
    /// Number of satellites in use/visible (unclear).
    /// Example: `0`.
    sat: u8,
    /// Description of the data collection event to which this data belongs.
    /// Example: `""`, `"Hiking"`.
    desc: String,
    /// Altitude in meters, using WGS84. Note that MSL must be set false in settings.
    /// Example: `1387.0`.
    alt: f64,
    /// Estimate of the accuracy of the location in meters. This is presumed to be the horizontal
    /// accuracy (earth-tangent plane).
    /// Example: `6.0`.
    acc: f32,
    /// Direction in degrees (unknown whether 0 is north or east, presumed north). This is also
    /// presumably direction of travel (angle of velocity vector), but may be the fused estimate of
    /// phone orientation.
    /// Example: `170.8125`.
    dir: f32,
    /// Provider of the location data. Known possible values are:
    /// - `"gps"`: GPS location data
    prov: String,
    /// Speed in kilometers per hour.
    /// Example: `0.0`.
    spd_kph: f32,
    /// Speed in (meters per second?).
    /// Example: `0.0`.
    spd: f32,
    /// Unix timestamp of the data, second-precision.
    /// Example: `1736999691`.
    timestamp: u64,
    /// Time as an ISO 8601 string with offset.
    /// Example: `2025-01-15T20:54:51.000-07:00`.
    timeoffset: String,
    /// Time as an ISO 8601 string in UTC.
    /// Example: `2025-01-16T03:54:51.000Z`.
    time: String,
    /// Unix timestamp of the start of the data collection event, second-precision.
    /// Example: `1737000139`.
    starttimestamp: u64,
    /// Date as an ISO 8601 string.
    /// Example: `2025-01-16`.
    date: String,
    /// Battery percentage.
    /// Example: `27.0`.
    batt: f32,
    /// Whether the device is charging.
    /// Example: `false`.
    ischarging: bool,
    /// Android ID
    /// Example: `4ca9e1da592aca9b`.
    aid: String,
    /// Serial number
    /// Example: `4ca9e1da592aca9b`.
    ser: String,
    /// File name of the data collection event on the phone.
    /// Example: `20250115`.
    filename: String,
    /// Profile name of the data collection event on the phone.
    /// Example: `Default Profile`.
    profile: String,
    /// Horizontal dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    hdop: Option<f32>,
    /// Vertical dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    vdop: Option<f32>,
    /// Position dilution of precision. May not be present.
    /// Example: ``, `1.0`.
    #[serde(deserialize_with = "deserialize_option_f32")]
    pdop: Option<f32>,
    /// Distance traveled. Unclear whether this is distance from last data point, distance from
    /// last sent point, or distance since start of data collection event.
    /// Example: `0`.
    dist: f32,
}

impl Payload {
    /// Create a Payload struct from a HTTP Payload string.
    ///
    /// # Arguments
    /// * `body_str` - A string containing the body of a HTTP message.
    ///
    /// # Return
    /// Payload struct containing the parsed data.
    pub fn from_http_body(body_str: &String) -> Result<Payload> {
        serde_urlencoded::from_str(body_str)
            .map_err(|e| eyre!("Failed to parse body string: {}", e))
    }
}

// unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_http_body() {
        let body_str = "lat=41.74108695983887&lon=-91.84490871429443&sat=0&desc=&alt=1387.0&acc=6.0&dir=170.8125&prov=gps&spd_kph=0.0&spd=0.0&timestamp=1736999691&timeoffset=2025-01-15T20:54:51.000-07:00&time=2025-01-16T03:54:51.000Z&starttimestamp=1737000139&date=2025-01-16&batt=27.0&ischarging=false&aid=4ca9e1da592aca9b&ser=4ca9e1da592aca9b&act=&filename=20250115&profile=Default+Profile&hdop=&vdop=&pdop=&dist=0".to_string();
        let payload = Payload::from_http_body(&body_str).unwrap();
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
        assert_eq!(payload.timestamp, 1736999691);
        assert_eq!(payload.timeoffset, "2025-01-15T20:54:51.000-07:00");
        assert_eq!(payload.time, "2025-01-16T03:54:51.000Z");
        assert_eq!(payload.starttimestamp, 1737000139);
        assert_eq!(payload.date, "2025-01-16");
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
    }
}
