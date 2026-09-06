use crate::{AppState, models::bgp_visibility::BgpVisibility};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BgpVisibilityQuery {
    pub country: String,
}

fn row_to_visibility(row: &rusqlite::Row) -> rusqlite::Result<BgpVisibility> {
    Ok(BgpVisibility {
        id: row.get(0)?,
        country_code: row.get(1)?,
        date: row.get(2)?,
        registered_asns: row.get(3)?,
        routed_asns: row.get(4)?,
        registered_v4_prefixes: row.get(5)?,
        routed_v4_prefixes: row.get(6)?,
        registered_v6_prefixes: row.get(7)?,
        routed_v6_prefixes: row.get(8)?,
        source: row.get(9)?,
    })
}

const SELECT: &str = "SELECT id, country_code, date, registered_asns, routed_asns,
    registered_v4_prefixes, routed_v4_prefixes, registered_v6_prefixes, routed_v6_prefixes, source
    FROM bgp_prefix_visibility
    WHERE country_code = ?1 ORDER BY date ASC";

pub async fn list_bgp_visibility(
    State(state): State<AppState>,
    Query(params): Query<BgpVisibilityQuery>,
) -> Result<Json<Vec<BgpVisibility>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![params.country], row_to_visibility)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
