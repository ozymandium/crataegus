Crataegus
===

## To Do
- change primary key to composite (username, timestamp)
    - this will prevent duplicates, but inserts will need to be error handled for when it happens
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
- should accuracy

