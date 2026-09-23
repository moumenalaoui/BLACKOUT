//! Persists the satellite fetcher's per-source element cache (see
//! `fetchers::satellites::SourceCache`) to SQLite, so a process restart (or a
//! redeploy that lands inside CelesTrak's own ~2h rate-limit window) doesn't
//! throw away every satellite that was ever successfully fetched. Read back
//! once at startup to prime the in-memory cache/catalog; rewritten in full
//! after every refresh cycle that has anything to save.

use crate::AppState;
use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

/// `source_group` value for rows from the Starlink supplemental feed, which
/// isn't a CelesTrak "GROUP" query at all — kept in the same table/shape
/// rather than a second table, since it's cached identically to a group.
const SUPPLEMENTAL_SENTINEL: &str = "__starlink_supplemental__";

pub struct LoadedCache {
    pub groups: HashMap<String, Vec<sgp4::Elements>>,
    pub supplemental: Vec<sgp4::Elements>,
    /// When this snapshot was written — every row shares one value, since
    /// `replace_all` rewrites the whole table in a single transaction.
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Rewrites the whole table from the fetcher's current in-memory cache. A
/// group that failed this cycle simply keeps whatever rows it already had —
/// its slice in `groups` is untouched by the caller in that case, so this
/// just rewrites the same rows back.
pub fn replace_all(
    state: &AppState,
    groups: &HashMap<String, Vec<sgp4::Elements>>,
    supplemental: &[sgp4::Elements],
) -> Result<()> {
    let mut conn = state
        .lock()
        .map_err(|_| anyhow::anyhow!("db lock poisoned"))?;
    let now = crate::util::date::now_utc_iso();
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM satellite_elements", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO satellite_elements (source_group, norad_id, elements_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (group, elements) in groups {
            for el in elements {
                stmt.execute(rusqlite::params![
                    group,
                    el.norad_id as i64,
                    serde_json::to_string(el)?,
                    now,
                ])?;
            }
        }
        for el in supplemental {
            stmt.execute(rusqlite::params![
                SUPPLEMENTAL_SENTINEL,
                el.norad_id as i64,
                serde_json::to_string(el)?,
                now,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Reads the persisted cache back, e.g. at process startup. Rows with
/// unparseable JSON are skipped and logged rather than failing the whole
/// load — matches how `fetch_and_store` already skips an individual invalid
/// element set rather than discarding a whole cycle over one bad object.
pub fn load_all(conn: &Connection) -> Result<LoadedCache> {
    let mut stmt =
        conn.prepare("SELECT source_group, elements_json, updated_at FROM satellite_elements")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut groups: HashMap<String, Vec<sgp4::Elements>> = HashMap::new();
    let mut supplemental = Vec::new();
    let mut updated_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut skipped = 0usize;

    for (source_group, elements_json, updated_at_raw) in rows {
        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&updated_at_raw) {
            let parsed = parsed.with_timezone(&chrono::Utc);
            updated_at = Some(match updated_at {
                Some(current) if current >= parsed => current,
                _ => parsed,
            });
        }
        match serde_json::from_str::<sgp4::Elements>(&elements_json) {
            Ok(elements) if source_group == SUPPLEMENTAL_SENTINEL => supplemental.push(elements),
            Ok(elements) => groups.entry(source_group).or_default().push(elements),
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        eprintln!(
            "satellites: skipped {skipped} persisted element set(s) with invalid stored JSON"
        );
    }
    Ok(LoadedCache {
        groups,
        supplemental,
        updated_at,
    })
}
