//use nom_exif::*;
use std::collections::VecDeque;
use std::path::PathBuf;
use log::{info, debug};
use crate::schema::Location;
use color_eyre::eyre::{Result, eyre};

/// Iterator that recursively searches for Exif GPS data in the given directory.
pub struct Finder {
    to_visit: VecDeque<PathBuf>,
}

impl Finder {
    /// Create a new Finder that will search the given directory.
    pub fn new(dir: &PathBuf) -> Self {
        let mut to_visit = VecDeque::new();
        to_visit.push_back(dir.clone());
        Finder { to_visit }
    }
}

impl Iterator for Finder {
    type Item = Location;

    fn next(&mut self) -> Option<Location> {
        while let Some(path) = self.to_visit.pop_front() {
            if path.is_dir() {
                let entries = path.read_dir().ok()?;
                for entry in entries {
                    let entry = entry.ok()?;
                    self.to_visit.push_back(entry.path());
                }
            } else if path.is_file() {
                match get_location(&path) {
                    Some(location) => return Some(location),
                    None => continue,
                }
            }
        }
        None
    }
}

fn get_location(path: &PathBuf) -> Option<Location> {
    debug!("Getting location from file: {}", path.display());
    // always return None if any error arises.
    let ms = match nom_exif::MediaSource::file_path(&path.as_path()) {
        Ok(ms) => ms,
        Err(_) => {
            debug!("Error creating MediaSource from file: {}", path.display());
            return None;
        },
    };
    if !ms.has_exif() {
        debug!("File does not have EXIF data: {}", path.display());
        return None;
    }
    let mut parser = nom_exif::MediaParser::new();
    let iter: nom_exif::ExifIter = match  parser.parse(ms) {
        Ok(iter) => iter,
        //Err(_) => return None,
        Err(e) => {
            debug!("Error parsing EXIF data from file: {}", path.display());
            debug!("Error: {}", e);
            return None;
        },
    };
    let gps_info: nom_exif::GPSInfo = match iter.parse_gps_info() {
        Ok(maybe_gps_info) => match maybe_gps_info {
            Some(gps_info) => gps_info,
            None => return None,
        },
        //Err(_) => return None,
        Err(e) => {
            debug!("Error parsing GPSInfo from file: {}", path.display());
            debug!("Error: {}", e);
            return None;
        },
    };
    info!("Found GPSInfo in file: {}", path.display());
    match location_from_gps_info(&gps_info) {
        Ok(location) => Some(location),
        Err(e) => {
            debug!("Error creating Location from GPSInfo: {}", e);
            None
        },
    }
}

fn location_from_gps_info(gps_info: &nom_exif::GPSInfo) -> Result<Location> {
    use crate::ffi::{
        EPSG9705, EPSG4979, epsg4979_from_epsg9705,
    };
    // first, get height in WGS84 instead of MSL
    let epsg9705 = EPSG9705 {
        lat: dd_from_latlng_ref(&gps_info.latitude, gps_info.latitude_ref)?, // should be same
        lon: dd_from_latlng_ref(&gps_info.longitude, gps_info.longitude_ref)?, // should be same
        alt: gps_info.altitude.as_float() * f64::from(gps_info.altitude_ref), // msl
    };
    let mut epsg4979 = EPSG4979 {
        lat: 0.0,
        lon: 0.0,
        alt: 0.0,
    };
    let status = unsafe { epsg4979_from_epsg9705(&epsg9705, &mut epsg4979) };
    let alt = match status {
        0 => epsg4979.alt,
        -1 => return Err(eyre!("epsg4979_from_epsg9705: null pointer")),
        -2 => return Err(eyre!("epsg4979_from_epsg9705: context creation")),
        -3 => return Err(eyre!("epsg4979_from_epsg9705: transformation creation")),
        -4 => return Err(eyre!("epsg4979_from_epsg9705: transformation failure")),
        _ => return Err(eyre!("epsg4979_from_epsg9705: unknown error: {}", status)),
    };
    // now we need to look up local time from gps time.

    //// now, we're finally done
    //Ok(Location {
    //    latitude: epsg4979.lat,
    //    longitude: epsg4979.lon,
    //    altitude: epsg4979.alt,
    //    accuracy: None,
    //    time_utc: gps_info.time,
    //    time_local: 
    //})
    Err(eyre!("Not implemented"))
}

/// Convert lat/lng and a direction character to decimal degrees.
fn dd_from_latlng_ref(ll: &nom_exif::LatLng, ref_: char) -> Result<f64> {
    let deg: f64 = ll.0.as_float();
    let min: f64 = ll.1.as_float();
    let sec: f64 = ll.2.as_float();

    let dir_sign = match ref_ {
        'N' | 'E' => 1.0,
        'S' | 'W' => -1.0,
        _ => return Err(eyre!("Invalid direction character: {}", ref_)),
    };
    Ok(dir_sign * (deg + min / 60.0 + sec / 3600.0))
}
