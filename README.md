Crataegus
===
## To Do
- graceful decline to record duplicate entries
- should db decline to record if user doesn't exist?
    - in practice this can't happen because it's protected in the server layer
    - perhaps the check should be lower level, at the db layer
    - this would requires delaying auth until record time, meaning each route has to implement auth, or call a db method that does auth. that's a lot of repeated code.
    - other option is to do user lookup twice
- source:
    - GPSLOGGER
    - PHOTO
- photo import
    - will need to make accuracy optional
- get rid of unwrap / graceful error handling
- figure out sqlite backup
    - rusqlite has backup integration as a feature
    - how to integrate into restic?
- figure out if there's a more built-in way to parse the body, possibly using axum_serde crate and/or having the data as url parameters
- status code returns in the server
- input sanitization
- set log level on sqlx queries to debug
- check for duplicates before adding
- figure out how to access profile settings saved from GpsLogger
