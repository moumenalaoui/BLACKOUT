use crate::models::ixp::IxpStats;
use anyhow::Result;
use rusqlite::Connection;

/// The whole table — one row per country with at least one known IXP, never
/// filtered per-request.
pub fn all(conn: &Connection) -> Result<Vec<IxpStats>> {
    let mut stmt = conn.prepare(
        "SELECT country_code, ixp_count, total_net_count, largest_ixp_name, \
         largest_ixp_net_count, generated_at FROM ixp_stats",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(IxpStats {
                country_code: row.get(0)?,
                ixp_count: row.get(1)?,
                total_net_count: row.get(2)?,
                largest_ixp_name: row.get(3)?,
                largest_ixp_net_count: row.get(4)?,
                generated_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
