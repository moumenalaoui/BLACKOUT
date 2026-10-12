use axum::{
    Extension, Router,
    http::{HeaderValue, Method, header},
    response::Response,
    routing::get,
};
use rusqlite::Connection;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

mod api;
mod db;
// Retained but unrouted. `POST /api/evaluate` is not mounted on this public
// read-only deployment, which leaves the whole assessment engine unreachable
// and therefore "dead" to the compiler. It is kept compiling — and its
// snapshot test kept running — so the feature can be restored behind a guard
// without resurrecting deleted code.
#[allow(dead_code)]
mod engine;
mod fetchers;
mod models;
mod satellites;
mod snapshot;
mod util;

pub type AppState = Arc<Mutex<Connection>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load a .env into the process environment so CLOUDFLARE_API_TOKEN and
    // friends reach std::env::var below. dotenvy looks in the working
    // directory and then its parents, so `cargo run` from `backend/` picks up
    // the repo-root .env (or a backend/.env, which wins). A plain Rust binary
    // — unlike Vite — does not read .env on its own. `.ok()` keeps startup
    // working when no .env is present, which is the case in the container:
    // Docker Compose injects the variables directly.
    dotenvy::dotenv().ok();

    // Listener and CORS settings are validated before anything else happens.
    // A typo here exits the process, and under `restart: unless-stopped` that
    // is a restart loop — which must not also be a loop of seeding the
    // database and firing every fetcher (CelesTrak rate-limits repeats).
    let addr = listen_addr()?;
    let cors = cors_layer()?;

    // Defaults to the historical relative path so `cargo run` from `backend/`
    // keeps working untouched; in production this points at a mounted volume
    // (the Docker image sets /data/mena_ai.db).
    let db_path = std::env::var("DATABASE_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "mena_ai.db".to_string());

    // The directory holding the database is not guaranteed to exist: a fresh
    // volume, or a filesystem a platform rebuilds on every deploy, may not
    // have it yet. `Connection::open` does not create parent directories, so
    // without this it fails with "unable to open database file" before the
    // snapshot restore has anywhere to put anything.
    if let Some(parent) = std::path::Path::new(&db_path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("could not create `{}` for the database: {e}", parent.display())
        })?;
    }

    // Optional, Replit-only: pulls the database back from Replit App Storage
    // when the filesystem has been rebuilt under us. Inert unless SNAPSHOT_KEY
    // is set — a self-hosted deployment keeps the file on a volume instead.
    // Must run before the two lines below: `exists()` has to see the restored
    // file, and `Connection::open` would create an empty one. `None` means no
    // snapshots this run — see the module docs for the three reasons, only one
    // of which is a problem.
    let snapshotter = snapshot::restore_if_absent(&db_path).await;

    // Captured *before* `Connection::open`, which creates the file if absent —
    // afterwards there is no way to tell a restored database from a brand-new
    // one, and that distinction is the whole point of `report_persistence`.
    let db_existed = std::path::Path::new(&db_path).exists();

    let conn = Connection::open(&db_path)
        .map_err(|e| anyhow::anyhow!("could not open database at `{db_path}`: {e}"))?;
    println!("DB at {db_path}");
    db::init_schema(&conn)?;
    println!("DB initialized and seeded.");
    warn_missing_optional_tokens();
    util::http::report_identity();

    let state: AppState = Arc::new(Mutex::new(conn));
    report_persistence(&state, &db_path, db_existed);

    // The built SPA. Default is the repo layout relative to `backend/`, so a
    // local `npm run build` is served without configuration; in a container
    // this points wherever dist was copied.
    let static_dir = std::env::var("STATIC_DIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "../frontend/dist".to_string());
    if std::path::Path::new(&static_dir)
        .join("index.html")
        .exists()
    {
        println!("Serving SPA from {static_dir}");
    } else {
        eprintln!(
            "WARNING: no index.html under `{static_dir}` — the API will work but \
             the app will 404. Run `npm run build` in frontend/, or set STATIC_DIR."
        );
    }

    // Fetchers hit slow/rate-limited external APIs (OONI alone can take
    // minutes) — run them in the background instead of blocking server
    // startup on them. The server is fully usable immediately; fetched data
    // fills in as each source completes.
    let fetch_state = state.clone();
    tokio::spawn(async move {
        db::run_fetcher_loop(fetch_state).await;
    });

    // The hot in-memory catalog (`SatelliteCatalog`) is independent of
    // AppState/SQLite, and on its own (much shorter) cadence than the
    // fetchers above — see `satellites` module docs for why. It still gets
    // its own `AppState` clone, same as the two loops above, purely so its
    // refresh loop can persist/restore a durable copy of what it fetches.
    let satellite_catalog = satellites::new_catalog();
    let satellite_fetch_catalog = satellite_catalog.clone();
    let satellite_fetch_state = state.clone();
    tokio::spawn(async move {
        fetchers::satellites::run_catalog_refresh_loop(satellite_fetch_catalog, satellite_fetch_state)
            .await;
    });

    // Same AppState/SQLite state as the fetch loop above — just a much
    // shorter cadence, since HTTP/3 protocol-share and BGP prefix-visibility
    // are meant to be leading indicators, not caught up to 6 hours late.
    let precision_state = state.clone();
    tokio::spawn(async move {
        db::run_precision_fetcher_loop(precision_state).await;
    });

    // Durability, on its own cadence again — unrelated to any fetcher's, since
    // this is about surviving a redeploy rather than about any data source.
    // Absent entirely when snapshots are off, so local runs spawn nothing and
    // a failed restore cannot lead to an upload.
    if let Some(snapshotter) = snapshotter {
        let interval_state = state.clone();
        let interval_snapshotter = snapshotter.clone();
        tokio::spawn(async move {
            interval_snapshotter.run_loop(interval_state).await;
        });

        // The interval above only bounds what an unplanned crash costs; this
        // is what makes an ordinary redeploy lossless.
        let shutdown_state = state.clone();
        tokio::spawn(async move {
            snapshotter.run_shutdown_hook(shutdown_state).await;
        });
    }

    // Public and read-only. Every route below is a GET; `POST /api/evaluate`
    // is deliberately NOT mounted — it was the one write path (it persists
    // evidence rows) and nothing in the UI calls it, so on a public URL it
    // would be an anonymous write for no benefit. `api::evaluate` and the
    // whole `engine` module stay compiled and ready to re-route behind a
    // guard if the feature is ever wanted.
    //
    // CORS is off by default: the browser loads the app and calls the API from
    // the same origin, so there is no cross-origin request to permit. It is
    // only switched on (see `cors_layer`) when the frontend is hosted
    // elsewhere and CORS_ALLOWED_ORIGINS names it.
    let app = Router::new()
        .route("/health", get(api::health::health))
        .route("/health/live", get(api::health::live))
        .route("/api/countries", get(api::countries::list_countries))
        .route("/api/countries/:code", get(api::countries::get_country))
        .route("/api/geo", get(api::geo::list_geo))
        .route("/api/cables", get(api::cables::list_cables))
        .route("/api/ixp-stats", get(api::ixp::list_ixp_stats))
        .route("/api/models", get(api::models::list_models))
        .route("/api/signals", get(api::signals::list_signals))
        .route("/api/blocking", get(api::blocking::list_blocking))
        .route("/api/categories", get(api::categories::list_categories))
        .route("/api/timeline", get(api::timeline::list_timeline))
        .route("/api/tor-metrics", get(api::tor_metrics::list_tor_metrics))
        .route("/api/outages", get(api::outages::list_outages))
        .route("/api/rankings", get(api::rankings::list_rankings))
        .route(
            "/api/censorship-index",
            get(api::censorship_index::list_censorship_index),
        )
        .route(
            "/api/country-scores",
            get(api::country_scores::list_country_scores),
        )
        .route("/api/satellites", get(api::satellites::list_satellites))
        // Before the `:norad_id` route is irrelevant to matching (the two have
        // different segment counts), but it must come before `route_layer`
        // below so the catalog Extension reaches it.
        .route(
            "/api/satellites/status",
            get(api::satellites::satellites_status),
        )
        .route(
            "/api/satellites/:norad_id/orbit",
            get(api::satellites::satellite_orbit),
        )
        // Scoped to the satellite routes above (not the SPA fallback below)
        // via `route_layer`. The position/orbit handlers take this *instead*
        // of `State<AppState>`, since they have nothing to do with SQLite;
        // `satellites_status` takes both, because half of what it reports is
        // refresh history that only survives restarts by living in SQLite.
        .route_layer(Extension(satellite_catalog))
        // Placed after `route_layer` above on purpose: these handlers take
        // neither the satellite catalog nor `State`, and nothing is gained by
        // sitting inside that Extension's scope.
        .route("/api/methodology", get(api::methodology::list_methodology))
        .route(
            "/api/methodology/:slug",
            get(api::methodology::get_methodology),
        )
        .route(
            "/api/http-protocol-share",
            get(api::http_protocol_share::list_http_protocol_share),
        )
        .route(
            "/api/bgp-visibility",
            get(api::bgp_visibility::list_bgp_visibility),
        )
        // The built SPA. Anything not matching a route above falls through to
        // ServeDir, and anything ServeDir can't find falls through to
        // index.html so client-side routes and deep links resolve.
        .fallback_service(spa_service(&static_dir))
        .layer(axum::middleware::map_response(vary_on_encoding))
        // Applied last so it wraps every route above *and* the SPA fallback —
        // the static bundle is 13.7 MB, most of it Cesium, and was being
        // served uncompressed to every first-time visitor. Bundle files with
        // a build-time `.gz` (see `spa_service`) now pass through it as-is;
        // gzipping them here cost ~150 ms of CPU per visitor for Cesium.js.
        //
        // The default predicate skips bodies under 32 bytes and already-
        // compressed content types (images, video), so the JPEG textures and
        // the like are not re-compressed for nothing.
        //
        // Measured on /api/satellites: 2.36 MB -> 0.62 MB. That endpoint is
        // polled every 7s per open tab, so it dominates egress.
        .layer(CompressionLayer::new())
        .with_state(state.clone());
    let app = match cors {
        Some(cors) => app.layer(cors),
        None => app,
    };

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("could not listen on {addr}: {e}"))?;
    println!("Listening on {addr} (app + API, public read-only)");
    axum::serve(listener, app).await?;
    Ok(())
}

/// A non-empty, trimmed environment variable.
fn env_value(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Where to listen, from `BIND_ADDR` (an IP address, default `0.0.0.0`) and
/// `PORT` (default 3001, which is what the Vite dev proxy targets).
///
/// A value that does not parse is an error rather than a silent fallback: a
/// typo'd PORT that quietly binds 3001 instead looks healthy from inside the
/// container while the reverse proxy in front of it gets connection refused.
fn listen_addr() -> anyhow::Result<SocketAddr> {
    parse_listen_addr(env_value("BIND_ADDR").as_deref(), env_value("PORT").as_deref())
}

fn parse_listen_addr(bind: Option<&str>, port: Option<&str>) -> anyhow::Result<SocketAddr> {
    let ip = match bind {
        Some(raw) => raw.parse::<IpAddr>().map_err(|_| {
            anyhow::anyhow!("BIND_ADDR `{raw}` is not an IP address (e.g. 0.0.0.0, ::, 127.0.0.1)")
        })?,
        None => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    };
    let port = match port {
        Some(raw) => raw
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("PORT `{raw}` is not a valid port number"))?,
        None => 3001,
    };
    Ok(SocketAddr::new(ip, port))
}

/// Cross-origin access to the API, from `CORS_ALLOWED_ORIGINS`: a
/// comma-separated list of origins (`https://blackout.example.org`), or `*`
/// for any. Unset means no CORS headers at all, which is right whenever the
/// SPA is served by this same process.
///
/// Read-only methods only — every route is a GET. Malformed entries fail
/// startup, since an origin that can never match fails silently in the
/// browser instead.
fn cors_layer() -> anyhow::Result<Option<CorsLayer>> {
    let Some(raw) = env_value("CORS_ALLOWED_ORIGINS") else {
        return Ok(None);
    };
    let entries: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .collect();
    if entries.is_empty() {
        return Ok(None);
    }

    let allow = if entries == ["*"] {
        AllowOrigin::any()
    } else {
        let origins = entries
            .iter()
            .map(|entry| parse_origin(entry))
            .collect::<anyhow::Result<Vec<_>>>()?;
        AllowOrigin::list(origins)
    };

    println!("CORS enabled for: {}", entries.join(", "));
    Ok(Some(
        CorsLayer::new()
            .allow_origin(allow)
            .allow_methods([Method::GET, Method::HEAD]),
    ))
}

/// One `CORS_ALLOWED_ORIGINS` entry as the header value a browser would send:
/// scheme and host (and port) only, with no trailing slash or path.
fn parse_origin(entry: &str) -> anyhow::Result<HeaderValue> {
    let origin = entry.trim_end_matches('/');
    let bare = matches!(
        origin.split_once("://"),
        Some(("http" | "https", host)) if !host.is_empty() && !host.contains('/')
    );
    if !bare {
        anyhow::bail!(
            "CORS_ALLOWED_ORIGINS entry `{entry}` must be a bare origin like \
             https://example.org, or `*` on its own"
        );
    }
    HeaderValue::from_str(origin)
        .map_err(|_| anyhow::anyhow!("CORS_ALLOWED_ORIGINS entry `{entry}` is not valid"))
}

/// Says plainly, at boot, whether the satellite catalog survived.
///
/// This exists because the single most damaging failure mode in this
/// deployment is silent: if the database was not restored, every redeploy
/// starts from an empty one and the satellite catalog has to be rebuilt from
/// scratch — which, during a CelesTrak outage, means it comes back as a
/// fraction of its real size. The plain "DB at <path>" line above looks
/// identical either way.
///
/// The boot counter in `satellite_refresh_state` is what makes the two cases
/// distinguishable: it can only be absent on a database no process has ever
/// booted against. A second boot reporting #1 means the file this process
/// opened is not the file the last one wrote.
fn report_persistence(state: &AppState, db_path: &str, db_existed: bool) {
    let boot = match db::satellite_catalog::record_boot(state) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("WARNING: could not record the boot marker: {e:#}");
            return;
        }
    };
    let satellites = state
        .lock()
        .ok()
        .and_then(|conn| db::satellite_catalog::count(&conn).ok())
        .unwrap_or(0);

    if db_existed && boot > 1 {
        println!(
            "DB restored: existing database at {db_path} (boot #{boot}), \
             {satellites} satellite(s) in the persistent catalog"
        );
        return;
    }

    eprintln!("WARNING: DB at {db_path} has no previous state (boot #{boot}).");
    eprintln!(
        "WARNING: that is expected on a first-ever start, and a problem on any other — \
         it means the previous run's data is gone."
    );
    if env_value("SNAPSHOT_KEY").is_some() {
        eprintln!(
            "WARNING: SNAPSHOT_KEY is set, so this database is expected to come back from the \
             Replit App Storage snapshot. Check the `snapshot:` lines above — the restore must \
             not have reported a WARNING."
        );
    } else {
        eprintln!(
            "WARNING: if this is not a first start, DATABASE_PATH is not on persistent storage. \
             The Docker setup keeps it on the `blackout-data` volume at /data — see README \
             \"Self-hosting\"."
        );
    }
}

/// `ServeDir` for the built SPA, falling back to `index.html` so client-side
/// routes and refreshes on a deep link return the app instead of a 404.
///
/// `precompressed_gzip` serves the `.gz` sibling that the frontend build
/// writes (frontend/scripts/precompress.mjs) to clients that accept gzip, and
/// the plain file to everyone else or when no `.gz` exists.
fn spa_service(dir: &str) -> ServeDir<ServeFile> {
    ServeDir::new(dir)
        .precompressed_gzip()
        .fallback(ServeFile::new(format!("{dir}/index.html")))
}

/// Precompressed files leave `ServeDir` with `Content-Encoding` already set,
/// so `CompressionLayer` passes them through untouched — and it only adds
/// `Vary: accept-encoding` to responses it compresses itself. Without this, a
/// shared cache could hand the gzip variant to a client that never asked for
/// it. Responses that already carry `Vary` (the satellite handler sets its
/// own) are left alone.
async fn vary_on_encoding(mut res: Response) -> Response {
    let headers = res.headers_mut();
    if headers.contains_key(header::CONTENT_ENCODING) && !headers.contains_key(header::VARY) {
        headers.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    }
    res
}

/// The optional API tokens gate whole data sources, and their absence used to
/// surface only as one `eprintln!` from deep inside a background task — so a
/// deploy that forgot `PULSE_API_TOKEN` looked identical to "Pulse has no data
/// for these countries". Say it once, loudly, at boot.
fn warn_missing_optional_tokens() {
    let missing: Vec<(&str, &str)> = [
        (
            "PULSE_API_TOKEN",
            "Internet Resilience Index (all countries)",
        ),
        (
            "CLOUDFLARE_API_TOKEN",
            "Cloudflare Radar outage annotations + HTTP/3 protocol-share signal",
        ),
    ]
    .into_iter()
    .filter(|(key, _)| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_none()
    })
    .collect();

    if missing.is_empty() {
        return;
    }
    eprintln!("WARNING: {} optional API token(s) missing —", missing.len());
    for (key, effect) in missing {
        eprintln!("WARNING:   {key} unset -> no data for: {effect}");
    }
    eprintln!(
        "WARNING: those sources will render empty, not error. Set them in .env or the host environment."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_addr_defaults_to_all_interfaces_on_3001() {
        assert_eq!(parse_listen_addr(None, None).unwrap(), "0.0.0.0:3001".parse().unwrap());
    }

    #[test]
    fn listen_addr_takes_ipv4_and_ipv6_bind_addresses() {
        assert_eq!(
            parse_listen_addr(Some("127.0.0.1"), Some("8080")).unwrap(),
            "127.0.0.1:8080".parse().unwrap()
        );
        assert_eq!(parse_listen_addr(Some("::"), Some("80")).unwrap(), "[::]:80".parse().unwrap());
    }

    #[test]
    fn listen_addr_rejects_garbage_instead_of_falling_back() {
        assert!(parse_listen_addr(None, Some("80a")).is_err());
        assert!(parse_listen_addr(None, Some("70000")).is_err());
        assert!(parse_listen_addr(Some("localhost"), None).is_err());
    }

    #[test]
    fn origins_are_normalised_to_what_a_browser_sends() {
        assert_eq!(parse_origin("https://example.org/").unwrap(), "https://example.org");
        assert_eq!(parse_origin("http://localhost:5173").unwrap(), "http://localhost:5173");
    }

    #[test]
    fn origins_with_a_path_or_no_scheme_are_rejected() {
        assert!(parse_origin("example.org").is_err());
        assert!(parse_origin("https://example.org/app").is_err());
        assert!(parse_origin("ftp://example.org").is_err());
        assert!(parse_origin("https://").is_err());
    }
}
