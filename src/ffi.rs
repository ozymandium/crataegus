use std::os::raw::{c_double, c_int};

#[repr(C)]
pub struct EPSG9705 {
    pub lat: c_double,
    pub lon: c_double,
    pub alt: c_double,
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
