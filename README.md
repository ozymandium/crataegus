Crataegus
===
## To Do
- should db decline to record if user doesn't exist?
    - in practice this can't happen because it's protected in the server layer
    - perhaps the check should be lower level, at the db layer
    - this would requires delaying auth until record time, meaning each route has to implement auth, or call a db method that does auth. that's a lot of repeated code.
    - other option is to do user lookup twice
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
