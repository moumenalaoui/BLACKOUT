use crate::{AppState, models::ixp::IxpStats};
use axum::{Json, extract::State, http::StatusCode};

/// The whole table — a flat, generated lookup, not a per-country query. The
/// frontend fetches this once and indexes it client-side by country_code,
/// same convention as /api/starlink-status and /api/cables.
pub async fn list_ixp_stats(
    State(state): State<AppState>,
) -> Result<Json<Vec<IxpStats>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::db::ixp::all(&conn)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
