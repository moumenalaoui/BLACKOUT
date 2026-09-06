use serde::{Deserialize, Serialize};

/// One submarine cable feature from TeleGeography's public map. `id` is the
/// cable's own identifier — NOT the same as `feature_id`, the true per-row
/// primary key: a single logical cable can legitimately span two features
/// sharing one `id` (confirmed against the live dataset). `segments` is the
/// source `MultiLineString`'s array-of-linestrings, preserved as-is — never
/// flattened into one continuous line, since a break between segments can be
/// a real routing gap (or an antimeridian crossing TeleGeography already
/// split at), not something to draw a line across.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableRoute {
    pub id: String,
    pub feature_id: String,
    pub name: String,
    pub color: String,
    pub segments: Vec<Vec<[f64; 2]>>,
}

/// One cable landing point. `country_code` is resolved offline, once, by
/// `backend/scripts/gen_cable_data.mjs` (never at request time) — `None`
/// means the generator couldn't match the published location name against
/// any known country, not that the point itself is malformed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableLandingPoint {
    pub id: String,
    pub name: String,
    pub country_code: Option<String>,
    pub is_tbd: bool,
    pub lon: f64,
    pub lat: f64,
}
