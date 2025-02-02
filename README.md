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
