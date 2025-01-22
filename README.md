Crataegus
===
## To Do
- figure out sqlite backup
    - rusqlite has backup integration as a feature
    - how to integrate into restic?
- get rid of unwrap / graceful error handling
- photo import
    - harvest gps data from exif, set time from gps time and do a timezone lookup instead of using other image parameters for local time.
    - branch `exif` has started this
        - need to add timestamp parsing to the `nom-exif` crate
        - sometimes GPS seems "stuck" and the time/location for a photo are very old
            - only immediately obvious solution is to pull the local time out of the filename and skip if the gps time stamp has a large mismatch
- figure out if there's a more built-in way to parse the body, possibly using axum_serde crate and/or having the data as url parameters
    - url parameters is probably the better way.
- status code returns in the server
- input sanitization
    - sanity checks on data
- figure out how to access profile settings saved from GpsLogger

