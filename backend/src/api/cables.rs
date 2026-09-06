use crate::{
    AppState,
    models::cable::{CableLandingPoint, CableRoute},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

/// Both lists in one payload — the frontend's single toggle always wants
/// both together, so two round trips would buy nothing.
#[derive(Serialize)]
pub struct CablesResponse {
    pub routes: Vec<CableRoute>,
    pub landing_points: Vec<CableLandingPoint>,
}

pub async fn list_cables(
    State(state): State<AppState>,
) -> Result<Json<CablesResponse>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let routes =
        crate::db::cables::all_routes(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let landing_points = crate::db::cables::all_landing_points(&conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CablesResponse {
        routes,
        landing_points,
    }))
}
