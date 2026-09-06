use crate::{AppState, models::starlink_status::StarlinkStatus};
use axum::{Json, extract::State, http::StatusCode};

/// The whole table — a flat, hand-curated ~25-row lookup, not a per-country
/// query. The frontend fetches this once and indexes it client-side by
/// country_code, the same way it already does for /api/geo.
pub async fn list_starlink_status(
    State(state): State<AppState>,
) -> Result<Json<Vec<StarlinkStatus>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::db::starlink_status::all(&conn)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
