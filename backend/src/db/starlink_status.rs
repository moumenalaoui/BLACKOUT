use crate::models::starlink_status::StarlinkStatus;
use anyhow::Result;
use rusqlite::Connection;

const SELECT: &str = "SELECT country_code, status, confidence, note, source_note, last_reviewed
    FROM starlink_status";

fn row_to_status(row: &rusqlite::Row) -> rusqlite::Result<StarlinkStatus> {
    // `status` is stored as its serde SCREAMING_SNAKE_CASE text (e.g.
    // "BANNED") so it round-trips through JSON the same way sanctions_tier
    // does for `countries` — deserialize it back into the enum rather than
    // exposing the raw column as a bare string.
    let status_text: String = row.get(1)?;
    let status = serde_json::from_str(&format!("\"{status_text}\"")).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            format!("unrecognised starlink_status.status value: {status_text}").into(),
        )
    })?;
    Ok(StarlinkStatus {
        country_code: row.get(0)?,
        status,
        confidence: row.get(2)?,
        note: row.get(3)?,
        source_note: row.get(4)?,
        last_reviewed: row.get(5)?,
    })
}

/// The whole table — ~25 rows, never filtered. Countries with no row simply
/// don't appear; the frontend treats a lookup miss as "no known restriction".
pub fn all(conn: &Connection) -> Result<Vec<StarlinkStatus>> {
    let mut stmt = conn.prepare(SELECT)?;
    let rows = stmt
        .query_map([], row_to_status)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
