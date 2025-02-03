use std::path::Path;
use std::sync::Arc;

use chrono_english::parse_date_string;
use clap::ValueEnum;
use color_eyre::eyre::{eyre, Result};
use futures::StreamExt;
use inquire::{Password, Text};
use log::info;
use serde::Deserialize;

use crate::db::{Config as DbConfig, Db};
use crate::export::{create_exporter, Format as ExportFormat};
use crate::gpslogger::csv::read_csv;
use crate::server::{Config as ServerConfig, Server};

/// Configuration for the server, obtained from main.rs::Args
#[derive(Debug, Deserialize)]
pub struct Config {
    https: ServerConfig,
    db: DbConfig,
}

/// Types of supported imports
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ImportFormat {
    /// GPSLogger CSV format
    GpsLoggerCsv,
}

/// Implementation of the Config struct
impl Config {
    /// Load the configuration from a TOML file
    ///
    /// # Arguments
    /// * `path`: path to the TOML file
    ///
    /// # Returns
    /// The configuration struct
    pub fn load(path: &Path) -> Result<Config> {
        if !path.exists() {
            return Err(eyre!("Config file does not exist: {}", path.display()));
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| eyre!("Failed to read config file: {}", e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| eyre!("Failed to parse config file: {}", e))?;
        Ok(config)
    }
}

pub async fn serve(config: Config) -> Result<()> {
    info!("Starting Crataegus server");
    let db = Arc::new(
        Db::new(&config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let server =
        Server::new(config.https, db).map_err(|e| eyre!("Failed to create server: {}", e))?;
    server
        .serve()
        .await
        .map_err(|e| eyre!("Server failed: {}", e))?;
    Ok(())
}

pub async fn useradd(config: Config) -> Result<()> {
    println!("Adding a user to the database");
    let db = Arc::new(
        Db::new(&config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    println!("Connected to the database. Enter the user information:");
    let username = Text::new("Username").prompt()?;
    let password = Password::new("Password").prompt()?;
    db.user_insert(username, password)
        .await
        .map_err(|e| eyre!("Failed to add user: {}", e))?;
    println!("User added successfully");
    Ok(())
}

pub async fn backup(config: Config) -> Result<()> {
    println!("Backing up the database");
    let db = Arc::new(
        Db::new(&config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    db.backup()
        .await
        .map_err(|e| eyre!("Failed to backup database: {}", e))?;
    println!("Database backed up successfully");
    Ok(())
}

pub async fn export(
    config: Config,
    format: ExportFormat,
    path: &Path,
    username: &str,
    start_str: &str,
    stop_str: &str,
) -> Result<()> {
    let now = chrono::offset::Local::now().fixed_offset();
    let start = parse_date_string(start_str, now, chrono_english::Dialect::Us)
        .map_err(|_| eyre!("Failed to parse start date"))?;
    let stop = parse_date_string(stop_str, now, chrono_english::Dialect::Us)
        .map_err(|_| eyre!("Failed to parse stop date"))?;
    println!(
        "Exporting\n  format: {:?}\n  path: {}\n  start: {}\n  stop: {}",
        format,
        path.display(),
        start,
        stop
    );
    let db = Arc::new(
        Db::new(&config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let name = format!(
        "crataegus_export_{}_{}",
        start.to_rfc3339(),
        stop.to_rfc3339()
    );
    let mut exporter = create_exporter(format, &name, path)
        .map_err(|e| eyre!("Failed to create exporter: {}", e))?;
    let mut location_stream = db
        .location_stream(username, start.to_utc(), stop.to_utc())
        .await
        .map_err(|e| eyre!("Failed to get location stream: {}", e))?;
    let mut count = 0;
    while let Some(location) = location_stream.next().await {
        let location = location.map_err(|e| eyre!("A location in the stream failed: {}", e))?;
        exporter
            .write_location(&location)
            .map_err(|e| eyre!("Failed to write location: {}", e))?;
        count += 1;
    }
    exporter.finish()?;
    println!("Exported {} locations", count);
    Ok(())
}

async fn import_gps_logger_csv(db: Arc<Db>, path: &Path, username: &str) -> Result<(usize, usize)> {
    let mut added_count = 0;
    let mut skipped_count = 0;
    let iter = read_csv(path, username).map_err(|e| eyre!("Failed to read CSV file: {}", e))?;
    for location in iter {
        let location = location.map_err(|e| eyre!("Failed to read location: {}", e))?;
        match db
            .location_insert(location)
            .await
            .map_err(|e| eyre!("Failed to insert location: {}", e))?
        {
            true => added_count += 1,
            false => skipped_count += 1,
        }
    }
    Ok((added_count, skipped_count))
}

pub async fn import(
    config: Config,
    format: ImportFormat,
    path: &Path,
    username: &str,
) -> Result<()> {
    println!(
        "Importing\n  format: {:?}\n  path: {}",
        format,
        path.display()
    );
    let db = Arc::new(
        Db::new(&config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let (added_count, skipped_count) = match format {
        ImportFormat::GpsLoggerCsv => import_gps_logger_csv(db, path, username)
            .await
            .map_err(|e| eyre!("Failed to import GPSLogger CSV: {}", e))?,
    };
    println!(
        "Found {} locations. Added {}, skipped {}",
        added_count + skipped_count,
        added_count,
        skipped_count
    );
    Ok(())
}

//pub async fn info(config: Config, username: Option<&str>) -> Result<()> {
//    let db = Arc::new(
//        Db::new(&config.db)
//            .await
//            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
//    );
//    let users = db.user_vec().await?;
//    if let Some(username) = username {
//        let user = users
//            .iter()
//            .find(|user| user.username == username)
//            .ok_or_else(|| eyre!("User not found"))?;
//        println!(
//            "User: {}\n  Password: {}\n  Admin: {}",
//            user.username, user.password, user.admin
//        );
//    } else {
//        for user in users {
//            println!(
//                "User: {}\n  Password: {}\n  Admin: {}",
//                user.username, user.password, user.admin
//            );
//        }
//    }
//    Ok(())
//}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_import_gps_logger_csv() {
        static CSV_DATA: &str = r#"time,lat,lon,elevation,accuracy,bearing,speed,satellites,provider,hdop,vdop,pdop,geoidheight,ageofdgpsdata,dgpsid,activity,battery,annotation,timestamp_ms,time_offset,distance,starttimestamp_ms,profile_name,battery_charging
2025-01-24T07:02:29.168Z,24.240779519081116,-11.84485614299774,1476.0,48.0,,0.0,0,gps,,,,,,,,64,,1737702149168,2025-01-24T00:02:29.168-07:00,14780.376051140634,1737686054899,Default Profile,false
2025-01-24T07:23:55.551Z,24.241143584251404,-11.84490287303925,1411.0,48.0,,0.0,0,gps,,,,,,,,63,,1737703435551,2025-01-24T00:23:55.551-07:00,14821.04923758446,1737686054899,Default Profile,false
2025-01-24T07:30:20.375Z,24.241090416908264,-11.84478521347046,1355.0,48.0,,0.0,0,gps,,,,,,,,62,,1737703820375,2025-01-24T00:30:20.375-07:00,14832.590979680575,1737686054899,Default Profile,false
2025-01-24T07:36:54.148Z,24.24091112613678,-11.8446295261383,1414.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704214148,2025-01-24T00:36:54.148-07:00,14856.455069256903,1737686054899,Default Profile,false
2025-01-24T07:40:35.889Z,24.2408287525177,-11.84476947784424,1472.0,48.0,,0.0,0,gps,,,,,,,,62,,1737704435889,2025-01-24T00:40:35.889-07:00,14871.385559872322,1737686054899,Default Profile,false
2025-01-25T07:34:09.909Z,24.7410617163024,-11.84486579207021,1378.333910142936,7.7476687,,,0,gps,,,,,,,,60,,1737790449909,2025-01-25T00:34:09.909-07:00,7081.54436921358,1737783655597,Default Profile,false"#;
        static USERNAME: &str = "test";
        let dir = tempdir().unwrap();
        println!("tempdir: {:?}", dir.path());
        let csv_path = dir.path().join("test.csv");
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "{}", CSV_DATA).unwrap();
        let db_path = dir.path().join("test.db");
        let db_config = DbConfig {
            path: db_path,
            backups: 0,
        };
        let db = Arc::new(Db::new(&db_config).await.unwrap());
        db.user_insert(USERNAME.to_string(), "password".to_string())
            .await
            .unwrap();
        // insert the 3rd location to test that the import skips it and metrics are correct
        let loc3 = crate::schema::Location {
            username: USERNAME.to_string(),
            time_utc: chrono::DateTime::parse_from_rfc3339("2025-01-24T07:30:20.375Z")
                .unwrap()
                .into(),
            time_local: chrono::DateTime::parse_from_rfc3339("2025-01-24T00:30:20.375-07:00")
                .unwrap(),
            latitude: 24.241090416908264,
            longitude: -11.84478521347046,
            altitude: 1355.0,
            accuracy: Some(48.0),
            source: crate::schema::Source::GpsLogger,
        };
        db.location_insert(loc3.clone()).await.unwrap();
        // now import the CSV
        let (added_count, skipped_count) = import_gps_logger_csv(db.clone(), &csv_path, USERNAME)
            .await
            .unwrap();
        assert_eq!(added_count, 5);
        assert_eq!(skipped_count, 1);
        let locs = db
            .location_vec(
                USERNAME,
                chrono::DateTime::parse_from_rfc3339("2025-01-24T07:02:29.167Z")
                    .unwrap()
                    .into(),
                chrono::DateTime::parse_from_rfc3339("2025-01-25T07:34:09.910Z")
                    .unwrap()
                    .into(),
            )
            .await
            .unwrap();
        assert_eq!(locs.len(), 6);
        assert_eq!(locs[2], loc3);
    }
}
