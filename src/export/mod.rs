use crate::export::gpx::GpxExporter;
use crate::schema::Location;
use clap::ValueEnum;
use color_eyre::eyre::Result;
use std::path::PathBuf;
mod gpx;

/// Filtypes that can be exported
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Gpx,
}

/// Trait for exporting locations to a file.
pub trait Exporter {
    /// Write a location to the file
    /// # Arguments
    /// * `location`: The location to write
    /// # Returns
    /// Result indicating success or failure
    fn write_location(&mut self, location: &Location) -> Result<()>;

    /// Finish writing the file
    /// # Returns
    /// Result indicating success or failure
    /// # Note
    /// Failure to call this method may result in a corrupted file.
    fn finish(&mut self) -> Result<()>;
}

/// Exporter factory
/// # Arguments
/// * `format`: The format to export to
/// * `name`: Name of the track
/// * `path`: Path to the file to write
/// # Returns
/// The exporter
pub fn create_exporter(format: Format, name: &str, path: &PathBuf) -> Result<Box<dyn Exporter>> {
    match format {
        Format::Gpx => Ok(Box::new(GpxExporter::new(name, path)?)),
    }
}
