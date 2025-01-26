Crataegus
===
## To Do
- CSV Import from GPSLogger
- move some of the main.rs export code to the export module
- network locations from GpsLogger will be in MSL
- get rid of unwrap / graceful error handling
- photo import
    - harvest gps data from exif, set time from gps time and do a timezone lookup instead of using other image parameters for local time.
    - branch `exif` has started this
        - need to add timestamp parsing to the `nom-exif` crate
        - sometimes GPS seems "stuck" and the time/location for a photo are very old
            - only immediately obvious solution is to pull the local time out of the filename and skip if the gps time stamp has a large mismatch
        - required some system deps for PROJ lib, not great.
- status code returns in the server
- figure out how to access profile settings saved from GpsLogger
- Privacy/Security
    - user/pass are stored in plaintext.
    - input sanitization
- unit testing
    - server
    - gpx export

