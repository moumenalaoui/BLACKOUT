use serde::{Deserialize, Serialize};

/// Hand-curated, not a live measurement: no source publishes a structured
/// "Starlink is banned here" feed (Starlink's own map at starlink.com/map
/// shows availability, not an explicit ban flag). Absence of a
/// `StarlinkStatus` row for a country means "no known restriction" — there is
/// deliberately no `AVAILABLE` variant, so a lookup miss needs no UI
/// treatment beyond rendering nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StarlinkRestriction {
    /// Starlink's own GPS-geofence blocks the dish from operating at all.
    Banned,
    /// Not Starlink-geofenced, but the state jams the signal and/or
    /// criminalizes possession — a materially different kind of evidence
    /// than a platform-side block.
    Jammed,
    /// Regulatory approval pending, stalled, or otherwise unresolved.
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarlinkStatus {
    pub country_code: String,
    pub status: StarlinkRestriction,
    pub confidence: String,
    pub note: String,
    pub source_note: String,
    pub last_reviewed: String,
}
