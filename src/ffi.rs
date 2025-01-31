use color_eyre::eyre::{eyre, Result};

use std::os::raw::{c_double, c_int};

#[repr(C)]
pub struct EPSG9705 {
    pub lat: c_double,
    pub lon: c_double,
    pub msl: c_double,
}

#[repr(C)]
pub struct EPSG4979 {
    pub lat: c_double,
    pub lon: c_double,
    pub alt: c_double,
}

extern "C" {
    pub fn epsg4979_from_epsg9705(input: *const EPSG9705, output: *mut EPSG4979) -> c_int;
}

pub fn alt_wgs84_from_msl(input: &EPSG9705) -> Result<EPSG4979> {
    let mut output = EPSG4979 {
        lat: 0.0,
        lon: 0.0,
        alt: 0.0,
    };
    let ret = unsafe { epsg4979_from_epsg9705(input, &mut output) };
    match ret {
        0 => Ok(output),
        -1 => Err(eyre!("Nullpointer passed to _epsg4979_from_epsg9705")),
        -2 => Err(eyre!("PROJ context creation failed")),
        -3 => Err(eyre!("PROJ coordinate transformation creation failed")),
        -4 => Err(eyre!("PROJ coordinate transformation failed")),
        _ => Err(eyre!("Unknown error from _epsg4979_from_epsg9705: {}", ret)),
    }
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//    use pretty_assertions::assert_eq;
//
//    #[test]
//    fn test_alt_wgs84_from_msl() {
//        let input = EPSG9705 {
//            lat: 0.0,
//            lon: 0.0,
//            msl: 0.0,
//        };
//        let output = alt_wgs84_from_msl(&input).unwrap();
//        assert_eq!(output.lat, 0.0);
//        assert_eq!(output.lon, 0.0);
//        assert_eq!(output.alt, 0.0);
//    }
//}
