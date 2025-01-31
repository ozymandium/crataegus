use color_eyre::eyre::{eyre, Result, WrapErr};
use log::{debug, info};
use proj::{Proj, ProjBuilder};

pub(crate) struct Converter {
    proj_msl_to_wgs84_height: Proj,
}

impl Converter {
    pub fn new() -> Result<Self> {
        let mut builder = ProjBuilder::new();
        match builder.enable_network(true) {
            Ok(1) => debug!("Network enabled"),
            Ok(0) => return Err(eyre!("Request to enable network was corrupted")),
            Err(e) => return Err(e).wrap_err("Failed to enable network"),
            Ok(_) => {
                return Err(eyre!(
                    "Unknown error enabling network. Return code was not 0 or 1"
                ))
            }
        }
        builder.grid_cache_enable(true);
        let url_endpoint: String = builder
            .get_url_endpoint()
            .wrap_err("Failed to get URL endpoint")?;
        debug!("URL endpoint: {}", url_endpoint);
        assert!(builder.network_enabled());
        let proj_msl_to_wgs84_height = builder
            //.proj_known_crs("EPSG:4326+5714", "EPSG:4979", None) // same
            //.proj_known_crs("EPSG:4326+5773", "EPSG:4979", None) // different, but not expected
            .proj_known_crs("EPSG:4326+3855", "EPSG:4979", None)
            .wrap_err("Failed to build proj")?;
        Ok(Self {
            proj_msl_to_wgs84_height,
        })
    }

    pub fn convert(&self, lat_deg: f64, lon_deg: f64, msl_m: f64) -> Result<f64> {
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();
        let (_, _, alt_m) = self
            .proj_msl_to_wgs84_height
            .convert((lat_rad, lon_rad, msl_m))
            .wrap_err(format!(
                "Failed to convert [lat={} deg, lon={} deg, msl={} m]",
                lat_deg, lon_deg, msl_m
            ))?;
        Ok(alt_m)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_3d_crs_2() {
        // EPSG:5798 EGM84 height
        // EPSG:5773 EGM96 height
        // EPSG:3855 EGM2008 height
        // EPSG:5714 MSL height
        // EPSG:4326 WGS 84 (2D)
        // EPSG:4979 WGS 84 (3D)

        //let from = "EPSG:9705";
        //let from = "EPSG:4326+3855";
        //let from = "EPSG:4326+5773";
        //let to = "EPSG:4326+5978";
        //let to = "EPSG:4979";
        //let proj = Proj::new_known_crs(from, to, None).unwrap();
        //let t = proj.convert((39.903333, 116.391667, -9.2152)).unwrap();
        //
        let converter = super::Converter::new().unwrap();

        let alt = converter.convert(39.903333, 116.391667, -9.2152).unwrap();
        assert_relative_eq!(alt, -9.0975, epsilon = 1e-5);
    }
}
