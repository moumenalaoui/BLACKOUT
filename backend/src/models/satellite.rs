use chrono::{DateTime, Utc};
use serde::Serialize;

/// One satellite's position, propagated fresh to `SatellitesResponse::generated_at`.
#[derive(Debug, Clone, Serialize)]
pub struct SatellitePosition {
    pub norad_id: u64,
    pub name: String,
    pub category: String,
    pub lat: f64,
    pub lon: f64,
    pub alt_km: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SatellitesResponse {
    /// UTC instant every position in this response was propagated to.
    pub generated_at: DateTime<Utc>,
    /// When the orbital-element catalog itself was last successfully
    /// refreshed from CelesTrak. `None` before the first successful fetch.
    pub catalog_updated_at: Option<DateTime<Utc>>,
    pub satellites: Vec<SatellitePosition>,
}

/// One point on a sampled orbit path.
#[derive(Debug, Clone, Serialize)]
pub struct OrbitPoint {
    pub lat: f64,
    pub lon: f64,
    pub alt_km: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrbitResponse {
    pub norad_id: u64,
    pub name: String,
    pub period_minutes: f64,
    pub generated_at: DateTime<Utc>,
    /// Pre-split at the antimeridian so each inner vec can be drawn as its
    /// own polyline without a spurious wraparound line.
    pub segments: Vec<Vec<OrbitPoint>>,
}
