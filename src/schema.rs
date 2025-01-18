use color_eyre::eyre::Result;

pub use location::Model as Location;
pub use user::Model as User;

/// Trait applied to all models to allow one-line validation.
pub trait SanityCheck {
    fn sanity_check(&self) -> Result<()>;
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

pub mod location {
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::entity::prelude::*;

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
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

impl SanityCheck for Location {
    fn sanity_check(&self) -> Result<()> {
        use color_eyre::eyre::ensure;
        // TODO: validate user exists somehow here?
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
        Ok(())
    }
}

// TODO: Add user validation
