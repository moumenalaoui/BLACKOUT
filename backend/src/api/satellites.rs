//! Satellite position and orbit-path endpoints.
//!
//! Unlike every other handler in `api/`, these don't take `State<AppState>` —
//! there is no SQLite involved. They take an `Extension<SatelliteCatalog>`
//! instead (mounted in `main.rs` alongside, not instead of, `AppState`); see
//! `crate::satellites` for why.
//!
//! Positions are computed fresh on every request by propagating every cached
//! element set to "now" — there is no separate position cache to keep in
//! sync, and this endpoint is what a client is expected to poll every 5-10s
//! for visibly-moving satellites (see the frontend's polling `useEffect`).

use crate::models::satellite::{OrbitPoint, OrbitResponse, SatellitePosition, SatellitesResponse};
use crate::satellites::{SatelliteCatalog, geodetic, orbit};
use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;

/// Number of points sampled across one full orbital period for an orbit path.
/// 180 gives a visually smooth curve without an oversized response.
const ORBIT_SAMPLES: usize = 180;

#[derive(Deserialize)]
pub struct SatellitesQuery {
    /// Comma-separated category list (e.g. `starlink,gps`). Filtering
    /// happens here, before propagation, so an unrequested category is never
    /// computed or sent — bounds both payload size and CPU cost as the
    /// tracked category list grows.
    pub categories: Option<String>,
}

pub async fn list_satellites(
    Extension(catalog): Extension<SatelliteCatalog>,
    Query(params): Query<SatellitesQuery>,
) -> Result<Json<SatellitesResponse>, StatusCode> {
    let wanted = params.categories.map(|raw| {
        raw.split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    let guard = catalog
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = Utc::now();

    let mut satellites = Vec::new();
    for sat in guard.objects.iter() {
        if let Some(wanted) = &wanted
            && !wanted.iter().any(|c| c == &sat.category)
        {
            continue;
        }
        match geodetic::propagate_geodetic(&sat.elements, &sat.constants, now) {
            Ok((lat, lon, alt_km)) => satellites.push(SatellitePosition {
                norad_id: sat.norad_id,
                name: sat.name.clone(),
                category: sat.category.clone(),
                lat,
                lon,
                alt_km,
            }),
            Err(e) => {
                eprintln!(
                    "satellites: propagation failed for NORAD {}: {e}",
                    sat.norad_id
                )
            }
        }
    }

    Ok(Json(SatellitesResponse {
        generated_at: now,
        catalog_updated_at: guard.catalog_updated_at,
        satellites,
    }))
}

pub async fn satellite_orbit(
    Extension(catalog): Extension<SatelliteCatalog>,
    Path(norad_id): Path<u64>,
) -> Result<Json<OrbitResponse>, StatusCode> {
    let guard = catalog
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sat = guard
        .objects
        .iter()
        .find(|s| s.norad_id == norad_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = Utc::now();
    let period_minutes = orbit::period_minutes(sat.elements.mean_motion);
    let points = orbit::sample_orbit(
        &sat.elements,
        &sat.constants,
        now,
        period_minutes,
        ORBIT_SAMPLES,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let segments = orbit::split_at_antimeridian(points)
        .into_iter()
        .map(|segment| {
            segment
                .into_iter()
                .map(|(lat, lon, alt_km)| OrbitPoint { lat, lon, alt_km })
                .collect()
        })
        .collect();

    Ok(Json(OrbitResponse {
        norad_id: sat.norad_id,
        name: sat.name.clone(),
        period_minutes,
        generated_at: now,
        segments,
    }))
}
