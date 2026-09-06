//! TEME -> WGS84 geodetic conversion.
//!
//! `sgp4::Constants::propagate` returns a position in the TEME (True Equator,
//! Mean Equinox) frame, in km — it deliberately stops there and leaves frame
//! conversion to the caller (see the crate's own README). Turning that into a
//! lat/lon/alt a globe renderer can plot needs two steps: rotate TEME to
//! Earth-fixed ECEF by the Greenwich Mean Sidereal Time at the instant of
//! propagation, then convert ECEF to geodetic on the WGS84 ellipsoid. Neither
//! step is orbital propagation — the propagation itself is entirely the
//! `sgp4` crate's job.

use chrono::{DateTime, Utc};

/// WGS84 semi-major axis, km.
const WGS84_A_KM: f64 = 6378.137;
/// WGS84 flattening.
const WGS84_F: f64 = 1.0 / 298.257223563;

/// Propagates one element set to `at` and converts the result to geodetic
/// coordinates. Returns `(lat_deg, lon_deg, alt_km)`.
pub fn propagate_geodetic(
    elements: &sgp4::Elements,
    constants: &sgp4::Constants,
    at: DateTime<Utc>,
) -> Result<(f64, f64, f64), sgp4::Error> {
    let minutes = minutes_since_epoch(elements, at);
    let prediction = constants.propagate(sgp4::MinutesSinceEpoch(minutes))?;
    Ok(teme_to_geodetic(prediction.position, at))
}

/// Minutes elapsed between an element set's epoch and `at`. Negative before
/// the epoch — SGP4 propagates equally well a short distance backwards.
pub fn minutes_since_epoch(elements: &sgp4::Elements, at: DateTime<Utc>) -> f64 {
    (at.naive_utc() - elements.datetime).num_milliseconds() as f64 / 60_000.0
}

/// TEME position (km) at instant `at` -> WGS84 geodetic `(lat_deg, lon_deg,
/// alt_km)`. The TEME->ECEF step is a pure Z-axis rotation by GMST; this
/// ignores polar motion and nutation/precession corrections beyond what TEME
/// already bakes in, which is standard practice for SGP4-based tracking (SGP4
/// itself is only accurate to within about a kilometer, so sub-arcsecond
/// frame corrections wouldn't survive it anyway).
pub fn teme_to_geodetic(position_teme_km: [f64; 3], at: DateTime<Utc>) -> (f64, f64, f64) {
    let years_since_j2000 = sgp4::julian_years_since_j2000(&at.naive_utc());
    let gmst_rad = sgp4::iau_epoch_to_sidereal_time(years_since_j2000);
    let (sin_gmst, cos_gmst) = gmst_rad.sin_cos();

    let [x, y, z] = position_teme_km;
    let x_ecef = x * cos_gmst + y * sin_gmst;
    let y_ecef = -x * sin_gmst + y * cos_gmst;
    let z_ecef = z;

    ecef_to_geodetic(x_ecef, y_ecef, z_ecef)
}

/// ECEF (km) -> WGS84 geodetic, via Bowring-style fixed-point iteration.
/// Six iterations converge far past the precision that matters here (SGP4's
/// own error dwarfs it).
fn ecef_to_geodetic(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);

    let mut lat = z.atan2(p * (1.0 - e2));
    let mut alt = 0.0;
    for _ in 0..6 {
        let sin_lat = lat.sin();
        let n = WGS84_A_KM / (1.0 - e2 * sin_lat * sin_lat).sqrt();
        alt = p / lat.cos() - n;
        lat = z.atan2(p * (1.0 - e2 * n / (n + alt)));
    }

    (lat.to_degrees(), lon.to_degrees(), alt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// ISS (ZARYA), from the `sgp4` crate's own `examples/omm.rs` fixture.
    fn iss_elements() -> sgp4::Elements {
        serde_json::from_str(
            r#"{
                "OBJECT_NAME": "ISS (ZARYA)",
                "OBJECT_ID": "1998-067A",
                "EPOCH": "2020-07-12T21:16:01.000416",
                "MEAN_MOTION": 15.49507896,
                "ECCENTRICITY": 0.0001413,
                "INCLINATION": 51.6461,
                "RA_OF_ASC_NODE": 221.2784,
                "ARG_OF_PERICENTER": 89.1723,
                "MEAN_ANOMALY": 280.4612,
                "EPHEMERIS_TYPE": 0,
                "CLASSIFICATION_TYPE": "U",
                "NORAD_CAT_ID": 25544,
                "ELEMENT_SET_NO": 999,
                "REV_AT_EPOCH": 23600,
                "BSTAR": -3.1515e-5,
                "MEAN_MOTION_DOT": -2.218e-5,
                "MEAN_MOTION_DDOT": 0
            }"#,
        )
        .expect("fixture parses")
    }

    #[test]
    fn propagates_iss_to_a_plausible_leo_position_at_epoch() {
        let elements = iss_elements();
        let constants = sgp4::Constants::from_elements(&elements).expect("valid elements");
        let at = Utc.from_utc_datetime(&elements.datetime);

        let (lat, lon, alt_km) =
            propagate_geodetic(&elements, &constants, at).expect("propagates at epoch");

        assert!((-90.0..=90.0).contains(&lat), "lat {lat} out of range");
        assert!((-180.0..=180.0).contains(&lon), "lon {lon} out of range");
        // The ISS orbits at roughly 400-430 km; give generous slack since this
        // is a fixed historical element set, not a live one.
        assert!(
            (300.0..600.0).contains(&alt_km),
            "alt_km {alt_km} not in plausible ISS range"
        );
    }

    #[test]
    fn propagation_an_hour_later_moves_the_satellite() {
        let elements = iss_elements();
        let constants = sgp4::Constants::from_elements(&elements).expect("valid elements");
        let at = Utc.from_utc_datetime(&elements.datetime);
        let later = at + chrono::Duration::hours(1);

        let a = propagate_geodetic(&elements, &constants, at).expect("propagates");
        let b = propagate_geodetic(&elements, &constants, later).expect("propagates");

        assert_ne!(a, b, "position should differ an hour later");
    }

    #[test]
    fn ecef_to_geodetic_round_trips_a_known_point() {
        // A point on the equator, on the prime meridian, at 500km altitude.
        let (lat, lon, alt) = ecef_to_geodetic(WGS84_A_KM + 500.0, 0.0, 0.0);
        assert!((lat).abs() < 1e-6, "lat {lat}");
        assert!((lon).abs() < 1e-6, "lon {lon}");
        assert!((alt - 500.0).abs() < 1e-6, "alt {alt}");
    }
}
