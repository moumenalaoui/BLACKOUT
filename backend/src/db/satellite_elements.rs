//! **Legacy, read-only.** The previous satellite persistence layer, keyed on
//! `(source_group, norad_id)`.
//!
//! Superseded by `db::satellite_catalog`, which is keyed on `norad_id` alone.
//! The reason for the change is the bug this design made unavoidable: because
//! a group's rows were rewritten wholesale from the last response for that
//! group, catalog *membership* was a property of the most recent fetch. One
//! truncated `active` response therefore deleted every satellite missing from
//! it — the 16k-to-1.6k collapse.
//!
//! Only `load_all` remains, so `fetchers::satellites::migrate_legacy_elements`
//! can carry an existing deployment's catalog forward once. Nothing writes
//! this table any more. It is deliberately not dropped: the migration is then
//! reversible, and leaving a few MB of superseded rows costs nothing next to
//! being unable to recover if the import turns out to be wrong.

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
}

/// Reads the persisted cache back, e.g. at process startup. Rows with
/// unparseable JSON are skipped and logged rather than failing the whole
/// load — matches how `fetch_and_store` already skips an individual invalid
/// element set rather than discarding a whole cycle over one bad object.
pub fn load_all(conn: &Connection) -> Result<LoadedCache> {
    let mut stmt = conn.prepare("SELECT source_group, elements_json FROM satellite_elements")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut groups: HashMap<String, Vec<sgp4::Elements>> = HashMap::new();
    let mut supplemental = Vec::new();
    let mut skipped = 0usize;

    for (source_group, elements_json) in rows {
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
    })
}
