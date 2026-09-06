//! Orbit-period calculation and orbit-path sampling for a selected satellite.
//!
//! Neither of these is orbital propagation — the propagation itself is
//! entirely `sgp4::Constants::propagate`, called once per sample here. This
//! module only decides *which* instants to sample across one revolution and
//! how to cut the resulting polyline so it never draws a spurious line across
//! the whole globe when a pass crosses the antimeridian (the same problem, and
//! the same fix, as OSIRIS's `splitAtAntimeridian`).

use super::geodetic::propagate_geodetic;
use chrono::{DateTime, Utc};

/// One point on a sampled orbit path.
pub type OrbitPoint = (f64, f64, f64); // (lat_deg, lon_deg, alt_km)

/// Orbital period in minutes, from the OMM `MEAN_MOTION` field (revolutions
/// per day). No crate or CelesTrak-provided helper computes this — it is
/// plain arithmetic on an already-known field, not a hand-rolled propagator.
pub fn period_minutes(mean_motion_rev_per_day: f64) -> f64 {
    1440.0 / mean_motion_rev_per_day
}

/// Samples one full orbital period, centered on `at`, into `samples` points
/// (chronological order, half a period before `at` to half a period after).
/// Centering on "now" — rather than on the element set's epoch, as OSIRIS's
/// stale-marker workaround did — keeps the drawn path consistent with a
/// position that was itself just computed for "now".
pub fn sample_orbit(
    elements: &sgp4::Elements,
    constants: &sgp4::Constants,
    at: DateTime<Utc>,
    period_minutes: f64,
    samples: usize,
) -> Result<Vec<OrbitPoint>, sgp4::Error> {
    let samples = samples.max(2);
    let half_period = chrono::Duration::milliseconds((period_minutes * 60_000.0 / 2.0) as i64);
    let start = at - half_period;

    let mut points = Vec::with_capacity(samples);
    for i in 0..samples {
        let frac = i as f64 / (samples - 1) as f64;
        let t = start + chrono::Duration::milliseconds((period_minutes * 60_000.0 * frac) as i64);
        points.push(propagate_geodetic(elements, constants, t)?);
    }
    Ok(points)
}

/// Splits a chronological point sequence into segments that never cross the
/// antimeridian, so each segment can be drawn as its own polyline instead of
/// one that wraps the wrong way around the globe. A crossing is detected as a
/// jump in longitude greater than 180 degrees between consecutive points.
pub fn split_at_antimeridian(points: Vec<OrbitPoint>) -> Vec<Vec<OrbitPoint>> {
    let mut segments: Vec<Vec<OrbitPoint>> = Vec::new();
    let mut current: Vec<OrbitPoint> = Vec::new();

    for point in points {
        if let Some(&(_, prev_lon, _)) = current.last()
            && (point.1 - prev_lon).abs() > 180.0
        {
            segments.push(std::mem::take(&mut current));
        }
        current.push(point);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_minutes_matches_known_leo_value() {
        // The ISS fixture's mean motion (~15.495 rev/day) is a ~93 minute orbit.
        let minutes = period_minutes(15.49507896);
        assert!(
            (90.0..96.0).contains(&minutes),
            "period {minutes} out of range"
        );
    }

    #[test]
    fn split_at_antimeridian_keeps_a_non_crossing_path_whole() {
        let points: Vec<OrbitPoint> =
            vec![(0.0, 10.0, 500.0), (1.0, 15.0, 500.0), (2.0, 20.0, 500.0)];
        let segments = split_at_antimeridian(points);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 3);
    }

    #[test]
    fn split_at_antimeridian_breaks_on_a_wraparound_jump() {
        let points: Vec<OrbitPoint> = vec![
            (0.0, 170.0, 500.0),
            (1.0, 179.0, 500.0),
            (2.0, -179.0, 500.0), // crosses +180/-180
            (3.0, -170.0, 500.0),
        ];
        let segments = split_at_antimeridian(points);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].len(), 2);
        assert_eq!(segments[1].len(), 2);

        for segment in &segments {
            for pair in segment.windows(2) {
                assert!(
                    (pair[1].1 - pair[0].1).abs() <= 180.0,
                    "segment still contains a >180 degree jump"
                );
            }
        }
    }

    #[test]
    fn split_at_antimeridian_handles_empty_and_single_point_input() {
        assert!(split_at_antimeridian(vec![]).is_empty());
        let one = split_at_antimeridian(vec![(0.0, 0.0, 500.0)]);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].len(), 1);
    }
}
