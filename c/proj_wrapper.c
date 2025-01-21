// Note: A separate C header is not required since Rust will declare the function and structs.
#include <proj.h>
#include <math.h>

/// \brief LLA coordinates in EPSG:9705 (WGS84 Lat/Lon + MSL)
typedef struct {
    /// Latitude in degrees
    double lat;
    /// Longitude in degrees
    double lon;  
    /// Altitude above MSL in meters
    double alt;  
} EPSG9705;

/// \brief LLA coordinates in EPSG:4979 (WGS84 3D)
typedef struct {
    /// Latitude in degrees
    double lat;   
    /// Longitude in degrees
    double lon; 
    /// Altitude above the geoid in meters (WGS84)
    double alt;  
} EPSG4979;

/// \brief Converts coordinates from EPSG:9705 (WGS84 Lat/Lon + MSL) to EPSG:4979 (WGS84 3D)
/// \param input Pointer to input coordinates in EPSG:9705
/// \param output Pointer to output coordinates in EPSG:4979
/// \return 0 on success, negative value on error.
///     -1: Null pointer error
///     -2: Context creation failed
///     -3: Transformation creation failed
///     -4: Transformation failed
int epsg4979_from_epsg9705(const EPSG9705 *input, EPSG4979 *output) {
    if (!input || !output) return -1; // Null pointer error

    PJ_CONTEXT *ctx = proj_context_create();
    if (!ctx) return -2; // Context creation failed

    // Transformation from EPSG:9705 (MSL) to EPSG:4979 (WGS84 3D)
    PJ *P = proj_create_crs_to_crs(ctx, "EPSG:9705", "EPSG:4979", NULL);
    if (!P) {
        proj_context_destroy(ctx);
        return -3; // Transformation creation failed
    }

    // Convert degrees to radians
    double lat_rad = input->lat * M_PI / 180.0;
    double lon_rad = input->lon * M_PI / 180.0;

    // Input coordinates
    PJ_COORD c_in = proj_coord(lon_rad, lat_rad, input->alt, 0);

    // Perform transformation
    PJ_COORD c_out = proj_trans(P, PJ_FWD, c_in);

    if (proj_errno(P) != 0) {
        proj_destroy(P);
        proj_context_destroy(ctx);
        return -4; // Transformation failed
    }

    // Convert radians back to degrees
    output->lat = c_out.lpzt.phi * 180.0 / M_PI;
    output->lon = c_out.lpzt.lam * 180.0 / M_PI;
    output->alt = c_out.lpzt.z;

    // Clean up
    proj_destroy(P);
    proj_context_destroy(ctx);

    return 0; // Success
}
