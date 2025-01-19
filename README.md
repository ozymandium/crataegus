Crataegus
===
## To Do
- source:
    - GPSLOGGER
    - PHOTO
- photo import
    - harvest gps data from exif, set time from gps time and do a timezone lookup instead of using other image parameters for local time.
- get rid of unwrap / graceful error handling
- figure out sqlite backup
    - rusqlite has backup integration as a feature
    - how to integrate into restic?
- figure out if there's a more built-in way to parse the body, possibly using axum_serde crate and/or having the data as url parameters
- status code returns in the server
- input sanitization
    - sanity checks on data
- figure out how to access profile settings saved from GpsLogger

Schema changes:
- add source
