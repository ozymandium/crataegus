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
use color_eyre::eyre::{eyre, Result};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

pub struct GpxExporter {
    writer: BufWriter<File>,
}

impl GpxExporter {
    pub fn new(name: &String, path: &PathBuf) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let binding = HEADER_FMT.replace("{track_name}", name);
        let header = binding.trim();
        writer.write_all(header.as_bytes())?;
        Ok(GpxExporter { writer })
    }
}

impl Exporter for GpxExporter {
    fn write_location(&mut self, location: &Location) -> Result<()> {
        let binding = POINT_FMT
            .replace("{latitude}", &location.latitude.to_string())
            .replace("{longitude}", &location.longitude.to_string())
            .replace("{altitude}", &location.altitude.to_string())
            .replace("{time}", &location.time_local.to_rfc3339());
        let point = binding.trim();
        self.writer.write_all(point.as_bytes())?;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.writer.write_all(FOOTER.trim().as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
}

static HEADER_FMT: &str = r#"
<?xml version="1.0" encoding="UTF-8"?>
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
