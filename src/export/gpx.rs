/* This is an example GPX file.

<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="crataegus" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>Track Name</name>
    <trkseg>
      <trkpt lat="48.1173" lon="11.5167">
        <ele>545.4</ele>
        <time>2023-10-07T12:35:19Z</time>
      </trkpt>
      <trkpt lat="48.1172" lon="11.5168">
        <ele>546.0</ele>
        <time>2023-10-07T12:35:29Z</time>
      </trkpt>
      <trkpt lat="48.1175" lon="11.5166">
        <ele>547.5</ele>
        <time>2023-10-07T12:35:39Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>

*/
use crate::{export::Exporter, schema::Location};
use color_eyre::eyre::Result;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

/// Writes a GPX file piecewise. XML is written in chunks to avoid having to keep the entire file
/// in memory. This is a bit hacky, but a stream can be handled one line at a time, which is not
/// possible with existing XML libraries. Writes the header, then locations, then the footer, all
/// in sequence. Failure to call `finish` may result in a corrupted file.
pub struct GpxExporter {
    writer: BufWriter<File>,
}

impl GpxExporter {
    /// Create a new GPX exporter and writes the header to the file.
    /// # Arguments
    /// * `name`: The name of the track
    /// * `path`: The path to the file to write
    /// # Returns
    /// The exporter
    pub fn new(name: &str, path: &Path) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let header = HEADER_FMT.replace("{track_name}", name);
        writer.write_all(header.as_bytes())?;
        Ok(GpxExporter { writer })
    }
}

impl Exporter for GpxExporter {
    fn write_location(&mut self, location: &Location) -> Result<()> {
        let point = POINT_FMT
            .replace("{latitude}", &location.latitude.to_string())
            .replace("{longitude}", &location.longitude.to_string())
            .replace("{altitude}", &location.altitude.to_string())
            .replace("{time}", &location.time_local.to_rfc3339());
        self.writer.write_all(point.as_bytes())?;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.writer.write_all(FOOTER.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
}

static HEADER_FMT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="crataegus" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>{track_name}</name>
    <trkseg>
"#;

static POINT_FMT: &str = r#"
      <trkpt lat="{latitude}" lon="{longitude}">
        <ele>{altitude}</ele>
        <time>{time}</time>
      </trkpt>
"#;

static FOOTER: &str = r#"
    </trkseg>
  </trk>
</gpx>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Location, Source};
    use chrono::DateTime;
    use pretty_assertions::assert_eq;
    use std::fs::File;
    use std::io::{BufReader, Read};

    #[test]
    fn test_gpx_exporter() {
        let tempfile = tempfile::NamedTempFile::new().unwrap();
        {
            let mut exporter =
                GpxExporter::new(&"test".to_string(), &tempfile.path().to_path_buf()).unwrap();
            exporter
                .write_location(&Location {
                    username: "test".to_string(),
                    time_utc: DateTime::parse_from_rfc3339("2023-10-07T12:35:19Z")
                        .unwrap()
                        .into(),
                    time_local: DateTime::parse_from_rfc3339("2023-10-07T12:35:19+02:00")
                        .unwrap()
                        .into(),
                    latitude: 48.1173,
                    longitude: 11.5167,
                    altitude: 545.4,
                    accuracy: None,
                    source: Source::GpsLogger,
                })
                .unwrap();
            exporter
                .write_location(&Location {
                    username: "test".to_string(),
                    time_utc: DateTime::parse_from_rfc3339("2023-10-07T12:35:29Z")
                        .unwrap()
                        .into(),
                    time_local: DateTime::parse_from_rfc3339("2023-10-07T12:35:29+02:00")
                        .unwrap()
                        .into(),
                    latitude: 48.1172,
                    longitude: 11.5168,
                    altitude: 546.0,
                    accuracy: None,
                    source: Source::GpsLogger,
                })
                .unwrap();
            exporter
                .write_location(&Location {
                    username: "test".to_string(),
                    time_utc: DateTime::parse_from_rfc3339("2023-10-07T12:35:39Z")
                        .unwrap()
                        .into(),
                    time_local: DateTime::parse_from_rfc3339("2023-10-07T12:35:39+02:00")
                        .unwrap()
                        .into(),
                    latitude: 48.1175,
                    longitude: 11.5166,
                    altitude: 547.5,
                    accuracy: None,
                    source: Source::GpsLogger,
                })
                .unwrap();
            exporter.finish().unwrap();
        }
        let file = File::open(tempfile.path()).unwrap();
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents).unwrap();
        println!("{}", contents);
        assert_eq!(
            contents,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="crataegus" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>test</name>
    <trkseg>

      <trkpt lat="48.1173" lon="11.5167">
        <ele>545.4</ele>
        <time>2023-10-07T12:35:19+02:00</time>
      </trkpt>

      <trkpt lat="48.1172" lon="11.5168">
        <ele>546</ele>
        <time>2023-10-07T12:35:29+02:00</time>
      </trkpt>

      <trkpt lat="48.1175" lon="11.5166">
        <ele>547.5</ele>
        <time>2023-10-07T12:35:39+02:00</time>
      </trkpt>

    </trkseg>
  </trk>
</gpx>
"#
        );
    }
}
