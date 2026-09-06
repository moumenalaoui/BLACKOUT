use crate::models::cable::{CableLandingPoint, CableRoute};
use anyhow::Result;
use rusqlite::Connection;

/// The whole table — 728 static rows, never filtered per-request. `geometry`
/// is stored as JSON text and deserialized back into the segment array here.
pub fn all_routes(conn: &Connection) -> Result<Vec<CableRoute>> {
    let mut stmt =
        conn.prepare("SELECT id, feature_id, name, color, geometry FROM cable_routes")?;
    let rows = stmt
        .query_map([], |row| {
            let geometry_json: String = row.get(4)?;
            let segments: Vec<Vec<[f64; 2]>> =
                serde_json::from_str(&geometry_json).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        "invalid cable_routes.geometry JSON".into(),
                    )
                })?;
            Ok(CableRoute {
                id: row.get(0)?,
                feature_id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                segments,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The whole table — 1,925 static rows, never filtered per-request.
pub fn all_landing_points(conn: &Connection) -> Result<Vec<CableLandingPoint>> {
    let mut stmt =
        conn.prepare("SELECT id, name, country_code, is_tbd, lon, lat FROM cable_landing_points")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CableLandingPoint {
                id: row.get(0)?,
                name: row.get(1)?,
                country_code: row.get(2)?,
                is_tbd: row.get::<_, i64>(3)? != 0,
                lon: row.get(4)?,
                lat: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
