//! Shared in-memory satellite state: the cached orbital-element catalog and
//! the pure propagation/geometry helpers used to turn it into positions.
//!
//! This deliberately does **not** go through `AppState`/SQLite the way every
//! other fetched data source in this backend does. That pattern persists
//! fetched rows into the shared `Arc<Mutex<Connection>>` and reads them back
//! with plain SQL; it fits data that changes hourly and is read occasionally.
//! Satellite positions are recomputed on *every* `GET /api/satellites`
//! request (by design — see the API handlers), so routing that traffic
//! through the one mutex guarding every other endpoint's database access would
//! serialize unrelated requests behind satellite work. A separate
//! `Arc<RwLock<...>>`, mounted as an Axum `Extension` alongside (not instead
//! of) `AppState`, avoids that contention without touching any existing
//! handler.

pub mod geodetic;
pub mod orbit;

use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};

/// Shared handle to the cached catalog. Cloning is cheap (just the `Arc`);
/// every handler and the background refresh loop hold their own clone.
pub type SatelliteCatalog = Arc<RwLock<CatalogInner>>;

pub fn new_catalog() -> SatelliteCatalog {
    Arc::new(RwLock::new(CatalogInner::default()))
}

#[derive(Default)]
pub struct CatalogInner {
    pub objects: Vec<CachedSatellite>,
    /// `None` until the first successful fetch cycle completes.
    pub catalog_updated_at: Option<DateTime<Utc>>,
}

/// One tracked object: its orbital elements plus the propagator constants
/// derived from them once (at catalog-refresh time), so a position request
/// only ever pays for `propagate`, not for re-deriving `Constants`.
pub struct CachedSatellite {
    pub norad_id: u64,
    pub name: String,
    pub category: String,
    pub elements: sgp4::Elements,
    pub constants: sgp4::Constants,
}
