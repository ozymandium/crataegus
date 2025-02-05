/// This is a stress test meant to reproduce a bug #4. The database file fails to open, and it's
/// believed this is due to problems with concurrent writes. This program replicates the issue by
/// spawning 100 threads, each of which writes 1000 locations to the database. The test is
/// considered successful if the database file can be opened after the test completes and all data
/// is present.
use crataegus::{
    schema::{Location, Source},
    db::{Db, Config},
};
use tempfile::NamedTempFile;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let db_file = NamedTempFile::new().unwrap();
    let db = Arc::new(Db::new(&Config {
        path: db_file.path().to_path_buf(),
        backups: 1,
    }).await.unwrap());
    db.user_insert("test".to_string(), "test".to_string()).await.unwrap();

    let mut handles = vec![];

    for i in 0..100 {
        let db = db.clone();
        let handle = tokio::spawn(async move {
            for j in 0..1000 {
                let time_utc = chrono::Utc::now();
                let time_local = time_utc.with_timezone(&chrono::FixedOffset::east_opt(2 * 3600).unwrap());
                db.location_insert(Location {
                    username: "test".to_string(),
                    latitude: 0.0,
                    longitude: 0.0,
                    altitude: 0.0,
                    time_utc: time_utc,
                    time_local: time_local,
                    source: Source::GpsLogger,
                    accuracy: None,
                }).await.unwrap();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(
        db.location_count(None).await.unwrap(),
        100 * 1000,
    );
}
