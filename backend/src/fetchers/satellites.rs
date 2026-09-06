use crate::satellites::{CachedSatellite, SatelliteCatalog};
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

// CelesTrak's modern GP data endpoint. `FORMAT` must be given explicitly —
// CSV became the default response format (2026-05-09) when it's omitted.
// Query params must be uppercase.
const GP_ENDPOINT: &str = "https://celestrak.org/NORAD/elements/gp.php";

// Starlink-specific SpaceX-derived ephemeris, refreshed roughly daily and
// fresher than the generic GP sweep for newly-deployed satellites. Uses the
// same NORAD catalog numbers as the general GP catalog, so it can override a
// general-GP record for the same object rather than being a separate object.
const SUPPLEMENTAL_ENDPOINT: &str = "https://celestrak.org/NORAD/elements/supplemental/sup-gp.php";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
// CelesTrak's own data only updates roughly every 2 hours and firmly
// rate-limits repeat downloads of high-traffic groups (see 403 handling
// below) — this pacing is just about being a polite, sequential client
// across the handful of group requests in one cycle, not about the update
// cadence itself (that's `run_catalog_refresh_loop`'s job).
const REQUEST_PACING: Duration = Duration::from_millis(500);

const DEFAULT_CATALOG_REFRESH_HOURS: f64 = 2.0;
const MIN_CATALOG_REFRESH_SECS: u64 = 60;

/// Default CelesTrak GP groups to fetch, each tagged with the category
/// assigned to any object first seen there. Order is precedence: earlier
/// entries win when the same NORAD ID appears in more than one group (e.g. a
/// Starlink satellite that also shows up in the generic `active` sweep).
/// `gps-ops`/`galileo`/`glo-ops`/`beidou` share one `navigation` tag (the four
/// GNSS constellations don't overlap in practice, so first-match-wins never
/// actually arbitrates between them) and `stations`/`science` share one
/// `stations` tag (space stations plus telescope/science craft — CelesTrak
/// has no combined group for that pairing, so this is two group fetches
/// feeding one category). `military`/`resource` are CelesTrak's real, public
/// "Miscellaneous Military" (~27 objects — most military/intel satellites
/// simply have no public elements at all) and "Earth Resources" (~181
/// objects) groups; both are honest counts, not padded to match any external
/// reference. Overridable via `SATELLITE_GROUPS`
/// (`group:category,group:category,...`) so the wider wishlist (OneWeb/
/// Iridium, weather, debris, ...) can be added later purely through
/// configuration.
const DEFAULT_GROUPS: &[(&str, &str)] = &[
    ("starlink", "starlink"),
    ("gps-ops", "navigation"),
    ("galileo", "navigation"),
    ("glo-ops", "navigation"),
    ("beidou", "navigation"),
    ("military", "military"),
    ("resource", "earthobs"),
    ("stations", "stations"),
    ("science", "stations"),
    ("geo", "geo"),
    ("active", "other"),
];

/// Per-source last-known-good elements, held across cycles by the loop below.
/// CelesTrak's rate limit (and any other fetch failure) is scoped to *one*
/// group — treating any single failure as "abort the whole cycle" meant one
/// rate-limited group (routinely `starlink`, since it's a high-traffic group)
/// blocked every *other* group from refreshing too, even though they had
/// nothing to do with it and fetched fine on their own. Caching per-source
/// results here means a failed group just falls back to its own last
/// successful fetch, while every other group still gets fresh data.
#[derive(Default)]
struct SourceCache {
    /// Group name -> its most recently successfully fetched elements.
    groups: HashMap<String, Vec<sgp4::Elements>>,
    supplemental: Vec<sgp4::Elements>,
}

/// Assembles the `(category, elements)` list `merge_sources` expects, in
/// configured precedence order, using whatever's currently cached for each
/// group — freshly fetched this cycle, or left over from the last cycle that
/// succeeded for it. A group with nothing cached yet (never fetched
/// successfully) simply contributes nothing this cycle, rather than blocking
/// the others.
fn groups_from_cache(
    groups_config: &[(String, String)],
    cache: &SourceCache,
) -> Vec<(String, Vec<sgp4::Elements>)> {
    groups_config
        .iter()
        .filter_map(|(group, category)| {
            cache
                .groups
                .get(group)
                .map(|els| (category.clone(), els.clone()))
        })
        .collect()
}

/// Total element count across every cached group plus supplemental — used to
/// decide whether there's anything at all to publish. Deliberately *not*
/// `groups.is_empty()`: a group whose fetch nominally "succeeds" with an
/// empty array (e.g. a misconfigured `SATELLITE_GROUPS` name CelesTrak still
/// accepts but matches nothing) would otherwise count as "has data" forever
/// after, silently defeating the whole safety net on every future cycle too.
fn total_cached_elements(
    groups: &[(String, Vec<sgp4::Elements>)],
    supplemental: &[sgp4::Elements],
) -> usize {
    groups.iter().map(|(_, els)| els.len()).sum::<usize>() + supplemental.len()
}

/// Refetches every configured source and merges whatever's now available
/// (freshly fetched this cycle, or `cache`'s last-known-good for any source
/// that failed) into the catalog. Only errors out — leaving the catalog
/// completely untouched — when literally no source has ever succeeded, since
/// there is nothing at all to publish in that case. Only called from
/// `run_catalog_refresh_loop` below (unlike the other fetchers' `fetch_and_store`,
/// which `db::run_fetchers` calls directly), so this stays module-private.
async fn fetch_and_store(catalog: &SatelliteCatalog, cache: &mut SourceCache) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let groups_config = configured_groups();
    let mut any_failed = false;

    for (group, _category) in &groups_config {
        match fetch_group(&client, group).await {
            Ok(elements) => {
                cache.groups.insert(group.clone(), elements);
            }
            Err(e) => {
                any_failed = true;
                eprintln!(
                    "satellites: group `{group}` fetch failed this cycle, reusing its previous \
                     data if any (other groups are unaffected): {e}"
                );
            }
        }
        tokio::time::sleep(REQUEST_PACING).await;
    }

    match fetch_starlink_supplemental(&client).await {
        Ok(elements) => cache.supplemental = elements,
        Err(e) => {
            any_failed = true;
            eprintln!(
                "satellites: starlink supplemental fetch failed this cycle, reusing its previous \
                 data if any: {e}"
            );
        }
    }

    let groups = groups_from_cache(&groups_config, cache);
    if total_cached_elements(&groups, &cache.supplemental) == 0 {
        anyhow::bail!("no satellite data available yet from any source");
    }

    let merged = merge_sources(groups, cache.supplemental.clone());

    let mut objects = Vec::with_capacity(merged.len());
    let mut skipped = 0usize;
    for (norad_id, category, elements) in merged {
        match sgp4::Constants::from_elements(&elements) {
            Ok(constants) => objects.push(CachedSatellite {
                norad_id,
                name: elements
                    .object_name
                    .clone()
                    .unwrap_or_else(|| format!("NORAD {norad_id}")),
                category,
                elements,
                constants,
            }),
            Err(e) => {
                skipped += 1;
                eprintln!("satellites: skipping NORAD {norad_id} (invalid elements): {e}");
            }
        }
    }
    if skipped > 0 {
        eprintln!("satellites: {skipped} object(s) skipped this cycle (invalid elements)");
    }

    let count = objects.len();
    let mut guard = catalog
        .write()
        .map_err(|_| anyhow::anyhow!("satellite catalog lock poisoned"))?;
    guard.objects = objects;
    guard.catalog_updated_at = Some(chrono::Utc::now());
    drop(guard);
    if any_failed {
        println!(
            "satellites: catalog refreshed, {count} object(s) (one or more sources reused \
             previous data this cycle — see warnings above)"
        );
    } else {
        println!("satellites: catalog refreshed, {count} object(s)");
    }
    Ok(())
}

/// Mirrors `db::run_fetcher_loop`'s spawn/loop/sleep/log shape, but runs
/// independently on its own cadence. CelesTrak's GP/SupGP data only updates
/// roughly every 2 hours — far more often than the 6-hour default the other
/// (SQLite-backed) fetchers share — so this can't just join their loop; it
/// needs its own `SATELLITE_CATALOG_REFRESH_HOURS` interval.
pub async fn run_catalog_refresh_loop(catalog: SatelliteCatalog) {
    let hours = std::env::var("SATELLITE_CATALOG_REFRESH_HOURS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|h| h.is_finite() && *h > 0.0)
        .unwrap_or(DEFAULT_CATALOG_REFRESH_HOURS);
    let secs = ((hours * 3600.0) as u64).max(MIN_CATALOG_REFRESH_SECS);
    let gap = Duration::from_secs(secs);
    println!("Satellite catalog refresh loop: every {hours}h ({secs}s)");

    // Owned by this loop, not the catalog itself: per-source last-known-good
    // data is this fetcher's own bookkeeping, not something a request handler
    // ever needs to read.
    let mut cache = SourceCache::default();

    loop {
        if let Err(e) = fetch_and_store(&catalog, &mut cache).await {
            eprintln!("WARNING: satellite catalog refresh failed, keeping previous catalog: {e:#}");
        }
        tokio::time::sleep(gap).await;
    }
}

/// `SATELLITE_GROUPS` env var, `group:category,group:category,...`, or the
/// built-in default list when unset/empty/unparseable.
fn configured_groups() -> Vec<(String, String)> {
    parse_groups_config(std::env::var("SATELLITE_GROUPS").ok().as_deref())
}

fn parse_groups_config(raw: Option<&str>) -> Vec<(String, String)> {
    let from_raw = raw.and_then(|v| {
        let parsed: Vec<(String, String)> = v
            .split(',')
            .filter_map(|entry| {
                let (group, category) = entry.split_once(':')?;
                let group = group.trim();
                let category = category.trim();
                if group.is_empty() || category.is_empty() {
                    return None;
                }
                Some((group.to_string(), category.to_string()))
            })
            .collect();
        (!parsed.is_empty()).then_some(parsed)
    });

    from_raw.unwrap_or_else(|| {
        DEFAULT_GROUPS
            .iter()
            .map(|(g, c)| (g.to_string(), c.to_string()))
            .collect()
    })
}

async fn fetch_group(client: &reqwest::Client, group: &str) -> Result<Vec<sgp4::Elements>> {
    let resp = client
        .get(GP_ENDPOINT)
        .query(&[("GROUP", group), ("FORMAT", "JSON")])
        .send()
        .await?;
    reject_or_parse(resp).await
}

async fn fetch_starlink_supplemental(client: &reqwest::Client) -> Result<Vec<sgp4::Elements>> {
    let resp = client
        .get(SUPPLEMENTAL_ENDPOINT)
        .query(&[("FILE", "starlink"), ("FORMAT", "JSON")])
        .send()
        .await?;
    reject_or_parse(resp).await
}

/// CelesTrak returns HTTP 403 with a "GP data has not updated since your last
/// successful download" message when a high-traffic group (`active`,
/// `starlink`) is re-requested before its ~2-hour update cycle has elapsed.
/// That is an expected, documented condition — not a real failure — so it is
/// worth its own clear error message rather than reading as a generic outage.
async fn reject_or_parse(resp: reqwest::Response) -> Result<Vec<sgp4::Elements>> {
    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        anyhow::bail!(
            "CelesTrak has no new data since the last download yet (403) — expected under its \
             rate-limit policy, will retry next cycle"
        );
    }
    let resp = resp.error_for_status()?;
    Ok(resp.json::<Vec<sgp4::Elements>>().await?)
}

/// Merges CelesTrak GP group responses into one deduplicated set, tagged by
/// category. Earlier `groups` entries win the category tag when a NORAD ID
/// appears in multiple groups. `supplemental` (Starlink's SpaceX-derived
/// ephemeris) always overrides a general-GP record for the same NORAD ID —
/// it's fresher — but keeps whatever category the GP sweep assigned (a
/// supplemental record's own classification field isn't a category); a
/// supplemental-only NORAD ID (not seen in any GP group) is tagged
/// `"starlink"` directly, since that's the only supplemental source fetched.
fn merge_sources(
    groups: Vec<(String, Vec<sgp4::Elements>)>,
    supplemental: Vec<sgp4::Elements>,
) -> Vec<(u64, String, sgp4::Elements)> {
    let mut by_id: HashMap<u64, (String, sgp4::Elements)> = HashMap::new();
    for (category, elements) in groups {
        for el in elements {
            by_id.entry(el.norad_id).or_insert((category.clone(), el));
        }
    }
    for el in supplemental {
        let category = by_id
            .get(&el.norad_id)
            .map(|(c, _)| c.clone())
            .unwrap_or_else(|| "starlink".to_string());
        by_id.insert(el.norad_id, (category, el));
    }
    by_id
        .into_iter()
        .map(|(id, (category, elements))| (id, category, elements))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(norad_id: u64) -> sgp4::Elements {
        serde_json::from_str(&format!(
            r#"{{
                "OBJECT_NAME": "TEST-{norad_id}",
                "OBJECT_ID": "2020-001A",
                "EPOCH": "2020-07-12T21:16:01.000416",
                "MEAN_MOTION": 15.49507896,
                "ECCENTRICITY": 0.0001413,
                "INCLINATION": 51.6461,
                "RA_OF_ASC_NODE": 221.2784,
                "ARG_OF_PERICENTER": 89.1723,
                "MEAN_ANOMALY": 280.4612,
                "EPHEMERIS_TYPE": 0,
                "CLASSIFICATION_TYPE": "U",
                "NORAD_CAT_ID": {norad_id},
                "ELEMENT_SET_NO": 999,
                "REV_AT_EPOCH": 23600,
                "BSTAR": -3.1515e-5,
                "MEAN_MOTION_DOT": -2.218e-5,
                "MEAN_MOTION_DDOT": 0
            }}"#
        ))
        .expect("fixture parses")
    }

    #[test]
    fn dedups_by_norad_id_preferring_the_first_matching_group() {
        let groups = vec![
            ("starlink".to_string(), vec![fixture(1), fixture(2)]),
            ("other".to_string(), vec![fixture(2), fixture(3)]),
        ];
        let merged = merge_sources(groups, vec![]);

        let mut by_id: HashMap<u64, String> =
            merged.into_iter().map(|(id, cat, _)| (id, cat)).collect();
        assert_eq!(by_id.remove(&1).as_deref(), Some("starlink"));
        // Seen in both groups — the first (`starlink`) group's tag wins.
        assert_eq!(by_id.remove(&2).as_deref(), Some("starlink"));
        assert_eq!(by_id.remove(&3).as_deref(), Some("other"));
        assert!(by_id.is_empty());
    }

    #[test]
    fn supplemental_overrides_a_general_gp_record_for_the_same_id() {
        let groups = vec![("starlink".to_string(), vec![fixture(100)])];
        let supplemental = vec![fixture(100)];

        let merged = merge_sources(groups, supplemental);
        assert_eq!(merged.len(), 1);
        // Still tagged with the GP group's category, even though the
        // element-set contents came from the supplemental fetch.
        assert_eq!(merged[0].1, "starlink");
    }

    #[test]
    fn supplemental_only_object_is_tagged_starlink() {
        let merged = merge_sources(vec![], vec![fixture(200)]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0, 200);
        assert_eq!(merged[0].1, "starlink");
    }

    #[test]
    fn total_cached_elements_counts_actual_objects_not_group_presence() {
        // The exact regression this guards against: a group that "succeeded"
        // with zero objects must not read as "we have data" — only actual
        // element counts should.
        let groups = vec![("starlink".to_string(), vec![])];
        assert_eq!(total_cached_elements(&groups, &[]), 0);

        let groups = vec![
            ("starlink".to_string(), vec![]),
            ("gps".to_string(), vec![fixture(1), fixture(2)]),
        ];
        assert_eq!(total_cached_elements(&groups, &[]), 2);

        assert_eq!(total_cached_elements(&[], &[fixture(1)]), 1);
        assert_eq!(total_cached_elements(&[], &[]), 0);
    }

    #[test]
    fn parse_groups_config_falls_back_to_defaults_when_absent_or_empty() {
        for raw in [None, Some(""), Some("garbage-with-no-colon")] {
            let groups = parse_groups_config(raw);
            assert_eq!(groups.len(), DEFAULT_GROUPS.len());
            assert_eq!(groups[0].0, "starlink");
        }
    }

    #[test]
    fn groups_from_cache_skips_a_group_with_nothing_cached_yet() {
        let groups_config = vec![
            ("starlink".to_string(), "starlink".to_string()),
            ("gps-ops".to_string(), "gps".to_string()),
        ];
        let mut cache = SourceCache::default();
        // Simulates gps-ops having succeeded at least once, starlink never
        // having succeeded yet (e.g. rate-limited on every attempt so far).
        cache.groups.insert("gps-ops".to_string(), vec![fixture(1)]);

        let result = groups_from_cache(&groups_config, &cache);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "gps");
    }

    #[test]
    fn groups_from_cache_reuses_previous_data_for_a_group_that_failed_this_cycle() {
        let groups_config = vec![
            ("starlink".to_string(), "starlink".to_string()),
            ("gps-ops".to_string(), "gps".to_string()),
        ];
        let mut cache = SourceCache::default();
        // A prior cycle populated both groups...
        cache
            .groups
            .insert("starlink".to_string(), vec![fixture(1)]);
        cache.groups.insert("gps-ops".to_string(), vec![fixture(2)]);
        // ...this cycle only refreshed gps-ops (starlink's fetch failed and
        // left its cache entry untouched, exactly as `fetch_and_store` does).
        cache
            .groups
            .insert("gps-ops".to_string(), vec![fixture(2), fixture(3)]);

        let result = groups_from_cache(&groups_config, &cache);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("starlink".to_string(), vec![fixture(1)]));
        assert_eq!(result[1].1.len(), 2);
    }

    #[test]
    fn parse_groups_config_parses_a_valid_override() {
        let groups = parse_groups_config(Some("oneweb:oneweb, iridium-NEXT:iridium"));
        assert_eq!(
            groups,
            vec![
                ("oneweb".to_string(), "oneweb".to_string()),
                ("iridium-NEXT".to_string(), "iridium".to_string()),
            ]
        );
    }
}
