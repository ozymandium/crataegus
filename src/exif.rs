use crate::schema::Location;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use color_eyre::eyre::{eyre, Result, WrapErr};
use exif::{Exif, In, Reader, Tag, Value};
use log::{debug, info};

use crate::proj::Converter;

use std::{collections::VecDeque, fs::File, io::BufReader, path::PathBuf};

/// Iterator that recursively searches for Exif GPS data in the given directory.
pub struct Finder {
    to_visit: VecDeque<PathBuf>,
    username: String,
}

impl Finder {
    /// Create a new Finder that will search the given directory.
    pub fn new(dir: &PathBuf, username: &String) -> Self {
        let mut to_visit = VecDeque::new();
        to_visit.push_back(dir.clone());
        Finder {
            to_visit,
            username: username.clone(),
        }
    }
}

impl Iterator for Finder {
    type Item = Location;

    fn next(&mut self) -> Option<Location> {
        while let Some(path) = self.to_visit.pop_front() {
            debug!("Visiting: {}", path.display());
            if path.is_dir() {
                let entries = path.read_dir().ok()?;
                for entry in entries {
                    let entry = entry.ok()?;
                    self.to_visit.push_back(entry.path());
                }
            } else if path.is_file() {
                match get_location(&path, &self.username) {
                    Some(location) => return Some(location),
                    None => continue,
                }
            }
        }
        None
    }
}

//let static tags = [
//    Tag::GPSDateStamp,
//    Tag::GPSTimeStamp,
//    Tag::GPSLatitude,
//    Tag::GPSLatitudeRef,
//    Tag::GPSLongitude,
//    Tag::GPSLongitudeRef,
//    Tag::GPSAltitude,
//    Tag::GPSAltitudeRef,
//]

/// Top level function to get a Location from an Exif file.
/// # Arguments
/// * `path` - The path to the file to read.
/// * `username` - The username to associate with the location.
/// # Return
/// A Location struct with the data from the file, or None if there's any errors, such as the file
/// not containing Exif data.
fn get_location(path: &PathBuf, username: &String) -> Option<Location> {
    debug!("Getting location from file: {}", path.display());
    let file = File::open(path).ok()?;
    let exif = Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .ok()?;

    let latitude: f64 = get_latitude(&exif)?;
    let longitude: f64 = get_longitude(&exif)?;

    let datetime_utc: DateTime<Utc> = get_datetime_utc(&exif)?;
    let datetime_local: DateTime<FixedOffset> =
        localtime_at(datetime_utc, latitude, longitude).ok()?;
    debug!("datetime_local: {:?}", datetime_local);

    let altitude_msl: f64 = get_altitude_msl(&exif)
        .wrap_err("Failed to get altitude")
        .ok()?;
    debug!("altitude_msl: {}", altitude_msl);
    let conv = Converter::new().ok()?;
    let altitude_wgs84: f64 = conv.convert(latitude, longitude, altitude_msl).ok()?;
    debug!("altitude_wgs84: {}", altitude_wgs84);

    None
}

/////////////////////////////////////////////////////////////
// second-level functions to retrive fields from Exif data //
/////////////////////////////////////////////////////////////

fn get_latitude(exif: &Exif) -> Option<f64> {
    if let Some(lat_val_field) = exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
        match &lat_val_field.value {
            Value::Rational(lat_val_vec_rational) => {
                if lat_val_vec_rational.len() != 3 {
                    return None;
                }
                let mut lat_val_vec: Vec<f64> = vec![0.0; 3];
                for (i, rational) in lat_val_vec_rational.iter().enumerate() {
                    lat_val_vec[i] = rational.to_f64();
                }
                debug!("lat_val_vec: {:?}", lat_val_vec);
                let lat_ref = exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY)?;
                let lat_dir = string_from_ascii(&lat_ref.value).ok()?;
                debug!("lat_dir: {}", lat_dir);
                let lat = dd_from_dms_ref(&lat_val_vec, lat_dir.chars().next()?).ok()?;
                debug!("lat: {}", lat);
                Some(lat)
            }
            _ => None,
        }
    } else {
        None
    }
}

fn get_longitude(exif: &Exif) -> Option<f64> {
    if let Some(lng_val_field) = exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
        match &lng_val_field.value {
            Value::Rational(lng_val_vec_rational) => {
                if lng_val_vec_rational.len() != 3 {
                    return None;
                }
                let mut lng_val_vec: Vec<f64> = vec![0.0; 3];
                for (i, rational) in lng_val_vec_rational.iter().enumerate() {
                    lng_val_vec[i] = rational.to_f64();
                }
                debug!("lng_val_vec: {:?}", lng_val_vec);
                let lng_ref = exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY)?;
                let lng_dir = string_from_ascii(&lng_ref.value).ok()?;
                debug!("lng_dir: {}", lng_dir);
                let lng = dd_from_dms_ref(&lng_val_vec, lng_dir.chars().next()?).ok()?;
                debug!("lng: {}", lng);
                Some(lng)
            }
            _ => None,
        }
    } else {
        None
    }
}

fn get_datetime_utc(exif: &Exif) -> Option<DateTime<Utc>> {
    let naive_date = get_date(&exif)?;
    debug!("naive_date: {:?}", naive_date);
    let naive_time = get_time(&exif)?;
    debug!("naive_time: {:?}", naive_time);
    Some(NaiveDateTime::new(naive_date, naive_time).and_utc())
}

fn get_date(exif: &Exif) -> Option<NaiveDate> {
    if let Some(date_field) = exif.get_field(Tag::GPSDateStamp, In::PRIMARY) {
        debug!("date_field: {:?}", date_field);
        let date_str = string_from_ascii(&date_field.value).ok()?;
        debug!("date_str: {}", date_str);
        NaiveDate::parse_from_str(&date_str, "%Y:%m:%d").ok()
    } else {
        None
    }
}

fn get_time(exif: &Exif) -> Option<NaiveTime> {
    if let Some(time_field) = exif.get_field(Tag::GPSTimeStamp, In::PRIMARY) {
        match &time_field.value {
            Value::Rational(vec_rational) => {
                if vec_rational.len() != 3 {
                    return None;
                }
                let mut time_vec: Vec<u32> = vec![0; 3];
                for (i, rational) in vec_rational.iter().enumerate() {
                    let float: f32 = rational.to_f32();
                    if float.fract() != 0.0 {
                        debug!("Non-integer rational in time field: {:?}", rational);
                        return None;
                    }
                    time_vec[i] = float as u32;
                }
                debug!("time_vec: {:?}", time_vec);
                Some(NaiveTime::from_hms(time_vec[0], time_vec[1], time_vec[2]))
            }
            _ => None,
        }
    } else {
        None
    }
}

fn get_altitude_msl(exif: &Exif) -> Result<f64> {
    let msl_val_field = exif
        .get_field(Tag::GPSAltitude, In::PRIMARY)
        .ok_or_else(|| eyre!("Failed to get GPSAltitude"))?;

    let msl_vec_rational = match &msl_val_field.value {
        Value::Rational(vec_rational) => vec_rational,
        _ => return Err(eyre!("Expected Rational, got {:?}", msl_val_field.value)),
    };
    if msl_vec_rational.len() != 1 {
        return Err(eyre!("Expected 1 Rational, got {}", msl_vec_rational.len()));
    }

    let msl_val: f64 = msl_vec_rational[0].to_f64();
    debug!("msl_val: {}", msl_val);

    let msl_ref = exif
        .get_field(Tag::GPSAltitudeRef, In::PRIMARY)
        .ok_or_else(|| eyre!("Failed to get GPSAltitudeRef"))?;
    let msl_dir: u32 = msl_ref
        .value
        .get_uint(0)
        .ok_or_else(|| eyre!("Failed to get GPSAltitudeRef"))?;
    debug!("msl_dir: {}", msl_dir);
    let sign = match msl_dir {
        0 => 1.0,
        1 => -1.0,
        _ => return Err(eyre!("Invalid MSL altitude direction: {}", msl_dir)),
    };
    Ok(sign * msl_val)
}

/////////////////////////////////////
// exif-rs compatibility functions //
/////////////////////////////////////

fn string_from_ascii(value: &Value) -> Result<String> {
    match value {
        Value::Ascii(vec_vec_u8) => {
            let mut result = String::new();
            for (i, vec_u8) in vec_vec_u8.iter().enumerate() {
                let s = String::from_utf8(vec_u8.clone()).map_err(|e| eyre!("Utf8Error: {}", e))?;
                if i > 0 {
                    result.push('\n'); // Add a separator if needed
                }
                result.push_str(&s);
            }
            Ok(result)
        }
        _ => Err(eyre!("Expected Ascii, got {:?}", value)),
    }
}

///////////////////////
// Generic Utilities //
///////////////////////

/// Convert lat/lng and a direction character to decimal degrees.
fn dd_from_dms_ref(dms: &Vec<f64>, ref_: char) -> Result<f64> {
    let deg: f64 = dms[0];
    let min: f64 = dms[1];
    let sec: f64 = dms[2];

    let dir_sign = match ref_ {
        'N' | 'E' => 1.0,
        'S' | 'W' => -1.0,
        _ => return Err(eyre!("Invalid direction character: {}", ref_)),
    };
    Ok(dir_sign * (deg + min / 60.0 + sec / 3600.0))
}

static TZ_FINDER: std::sync::LazyLock<tzf_rs::DefaultFinder> =
    std::sync::LazyLock::new(|| tzf_rs::DefaultFinder::new());

fn localtime_at(utc: DateTime<Utc>, lat: f64, lng: f64) -> Result<DateTime<FixedOffset>> {
    use chrono::{Offset, TimeZone};
    use chrono_tz::{Tz, TzOffset};
    use std::str::FromStr;

    let tz_str: &str = TZ_FINDER.get_tz_name(lng, lat);
    debug!("Timezone: {}", tz_str);
    let tz = chrono_tz::Tz::from_str(tz_str)
        .wrap_err(format!("Failed to parse timezone: {}", tz_str))?;
    debug!("Timezone: {:?}", tz);
    let offset: TzOffset = tz.offset_from_utc_datetime(&utc.naive_utc());
    debug!("Offset: {:?}", offset);
    let fixed_offset: FixedOffset = offset.fix();
    debug!("FixedOffset: {:?}", fixed_offset);
    Ok(utc.with_timezone(&fixed_offset))
}
