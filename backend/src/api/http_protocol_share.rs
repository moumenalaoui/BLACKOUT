use crate::{AppState, models::http_protocol_share::HttpProtocolShare};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct HttpProtocolShareQuery {
    pub country: String,
}

fn row_to_share(row: &rusqlite::Row) -> rusqlite::Result<HttpProtocolShare> {
    Ok(HttpProtocolShare {
        id: row.get(0)?,
        country_code: row.get(1)?,
        date: row.get(2)?,
        http1_pct: row.get(3)?,
        http2_pct: row.get(4)?,
        http3_pct: row.get(5)?,
        source: row.get(6)?,
    })
}

const SELECT: &str = "SELECT id, country_code, date, http1_pct, http2_pct, http3_pct, source
    FROM http_protocol_share
    WHERE country_code = ?1 ORDER BY date ASC";

pub async fn list_http_protocol_share(
    State(state): State<AppState>,
    Query(params): Query<HttpProtocolShareQuery>,
) -> Result<Json<Vec<HttpProtocolShare>>, StatusCode> {
    let conn = state
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = stmt
        .query_map(rusqlite::params![params.country], row_to_share)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(result))
}
