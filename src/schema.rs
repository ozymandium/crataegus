use color_eyre::eyre::{ensure, Result};

pub use location::Model as Location;
pub use location::Source;
pub use user::Model as User;

/// Trait applied to all models to allow one-line validation.
pub trait SanityCheck {
    /// Perform a sanity check on the model.
    /// # Returns
    /// Result indicating success or failure.
    fn sanity_check(&self) -> Result<()>;
}

/// Trait to convert a something to a Location struct.
pub trait LocationGen {
    /// Create a Location struct.
    /// # Arguments
    /// * `self` - The struct to convert.
    /// * `username` - The username to associate with the location.
    /// # Return
    /// A Location struct with the data from the struct.
    fn to_location(&self, username: &String) -> Location;
}

pub mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub username: String,
        pub password: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

impl SanityCheck for User {
    fn sanity_check(&self) -> Result<()> {
        ensure!(
            self.username.len() <= 32,
            format!("Username too long: {}", self.username)
        );
        ensure!(
            self.password.len() <= 64,
            format!("Password too long: {}", self.password)
        );
        Ok(())
    }
}

pub mod location {
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::entity::prelude::*;

    /// Source of the location data.
    #[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
    pub enum Source {
        /// crate::gpslogger::Payload
        #[sea_orm(string_value = "GPSLogger")]
        GpsLogger,
    }

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "locations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub username: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub time_utc: DateTime<Utc>,
        pub time_local: DateTime<FixedOffset>,
        pub latitude: f64,
        pub longitude: f64,
        pub altitude: f64,
        pub accuracy: Option<f32>,
        pub source: Source,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::user::Entity",
            from = "Column::Username",
            to = "super::user::Column::Username",
            on_update = "Cascade",
            on_delete = "Cascade"
        )]
        User,
    }

    impl Related<super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

impl SanityCheck for Location {
    fn sanity_check(&self) -> Result<()> {
        use chrono::Utc;
        // float nan/inf checks
        ensure!(
            self.latitude.is_finite(),
            format!("Latitude is not finite: {}", self.latitude)
        );
        ensure!(
            self.longitude.is_finite(),
            format!("Longitude is not finite: {}", self.longitude)
        );
        ensure!(
            self.altitude.is_finite(),
            format!("Altitude is not finite: {}", self.altitude)
        );
        ensure!(
            self.accuracy.is_none() || self.accuracy.unwrap().is_finite(),
            format!("Accuracy is not finite: {:?}", self.accuracy)
        );
        // Position value checks
        ensure!(
            -90.0 <= self.latitude && self.latitude <= 90.0,
            format!("Latitude out of bounds: {}", self.latitude)
        );
        ensure!(
            -180.0 <= self.longitude && self.longitude <= 180.0,
            format!("Longitude out of bounds: {}", self.longitude)
        );
        ensure!(
            -1000.0 <= self.altitude && self.altitude <= 10000.0,
            format!("Altitude out of bounds: {}", self.altitude)
        );
        ensure!(
            self.accuracy.is_none()
                || (0.0 <= self.accuracy.unwrap() && self.accuracy.unwrap() <= 100.0),
            format!("Accuracy out of bounds: {:?}", self.accuracy)
        );
        // utc and local time should be the same
        ensure!(
            self.time_utc == self.time_local.with_timezone(&Utc),
            format!(
                "Time UTC and Time Local are not the same: {:?} != {:?}",
                self.time_utc, self.time_local
            )
        );
        Ok(())
    }
}
