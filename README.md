Crataegus
===

:construction: **(very) Work in Progress** :construction:

This is a bare bones location history server, similar to Google Timeline, [Dawarich](https://dawarich.app/) or [OwnTracks](https://owntracks.org/).
The focus is on having a single binary, with everything distributed via Cargo instead of containerized runtimes.
CLI interfaces are preferred for administration, so any web interfaces that may or may not exist in the future would be purely for data visualization.

Crataegus is built to ingest history from [GPSLogger](https://gpslogger.app), which is available on [FDroid](https://f-droid.org/packages/com.mendhak.gpslogger/). 

## Features

- HTTPS logging server: live location recording via GPSLogger's "Custom URL" functionality.
- Multi-user
- Import
    - GPSLogger-formatted CSV is the only currently supported bulk import format.
    - Work on adding more input methods, such as EXIF harvesting from JPG/MP4/etc, is underway.
- Export
    - GPX is currently the only supported format. [GPXSee](https://www.gpxsee.org/) is the recommended viewer.
    - Other export formats, such as KML heatmaps are also in progress.
- Backup
    - SQLite snapshots are stored in the same directory as the database.
    - Auto-deletion of old snapshots.
- REST API: WIP

## To Do (for now)
- error running:

        Feb 01 17:31:29 possum crataegus[2056310]: The application panicked (crashed).
        Feb 01 17:31:29 possum crataegus[2056310]: Message:  called `Result::unwrap()` on an `Err` value:
        Feb 01 17:31:29 possum crataegus[2056310]:    0: Failed to insert location into database for unknown reason: Model { username: "roco", time_utc: 2025-02-02T00:31:25.661Z, time_local: 2025-02-01T17:31:25.661-07:00, latitude: 40.74104549093144, longitude: -111.84490499319296, altitude: 1398.9059597764328, accuracy: Some(76.45648), source: GpsLogger }
        Feb 01 17:31:29 possum crataegus[2056310]:    1: Execution Error: error returned from database: (code: 14) unable to open database file
        Feb 01 17:31:29 possum crataegus[2056310]:    2: error returned from database: (code: 14) unable to open database file
        Feb 01 17:31:29 possum crataegus[2056310]:    3: error returned from database: (code: 14) unable to open database file
        Feb 01 17:31:29 possum crataegus[2056310]:    4: (code: 14) unable to open database file
        Feb 01 17:31:29 possum crataegus[2056310]: Location:
        Feb 01 17:31:29 possum crataegus[2056310]:    /home/roco/.local/rust/cargo/git/checkouts/crataegus-54933e8cbc2396ab/d378ad8/src/db.rs:243
        Feb 01 17:31:29 possum crataegus[2056310]: Backtrace omitted. Run with RUST_BACKTRACE=1 environment variable to display it.
        Feb 01 17:31:29 possum crataegus[2056310]: Run with RUST_BACKTRACE=full to include source snippets.
        Feb 01 17:31:29 possum crataegus[2056310]: Location: /home/roco/.local/rust/cargo/git/checkouts/crataegus-54933e8cbc2396ab/d378ad8/src/server.rs:120
        Feb 01 17:31:29 possum crataegus[2056310]: Backtrace omitted. Run with RUST_BACKTRACE=1 environment variable to display it.
        Feb 01 17:31:29 possum crataegus[2056310]: Run with RUST_BACKTRACE=full to include source snippets.

    - file had same owner as process user, permissions `rw-r--r--`
- make the export command use `--flags` instead of positional args
- add an info command
- Documentation
- network locations from GpsLogger will be in MSL
- get rid of unwrap / graceful error handling
- photo import
    - harvest gps data from exif, set time from gps time and do a timezone lookup instead of using other image parameters for local time.
    - branch `exif` has started this
        - need to add timestamp parsing to the `nom-exif` crate
        - sometimes GPS seems "stuck" and the time/location for a photo are very old
            - only immediately obvious solution is to pull the local time out of the filename and skip if the gps time stamp has a large mismatch
        - required some system deps for PROJ lib, not great. Options:
            - Hide EXIF import behind a feature flag (dislike)
            - Implement the WGS84 calculations internally (meh)
            - Extend [georust/proj](https://github.com/georust/proj) to handle 3D instead of just 2D (a lot of work).
- status code returns in the server
- figure out how to access profile settings saved from GpsLogger
- Privacy/Security
    - user/pass are stored in plaintext.
    - input sanitization
- unit testing
    - server
    - gpx export

