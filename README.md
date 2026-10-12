# BLACKOUT

BLACKOUT is a read-only web app for exploring internet freedom, censorship, and network disruption by country. It combines a React/Cesium globe frontend with a Rust/Axum API, a local SQLite store, background fetchers, and a small set of seeded reference data.

## What the app does

- Renders a whole-world 3D globe with:
  - a composite censorship-index choropleth derived from V-Dem and RSF
  - OONI-based country blocking blooms
  - recent IODA internet-outage blooms
  - an optional submarine-cable overlay
  - live-polled satellite positions, with selectable categories and orbit paths
- Opens a country sidebar with per-country metrics for:
  - blocking status across AI access and circumvention tools
  - messaging app reachability
  - censored content categories
  - 90-day outage history
  - Tor relay and bridge usage
  - HTTP/1.x, HTTP/2, and HTTP/3 traffic share
  - BGP visibility
  - Internet Exchange Point density
  - Internet Society Pulse resilience scores
  - V-Dem and RSF freedom scores
- Shows a global "most censored countries" ranking panel and a footer with source links, link state, and data age.

Any drawable country on the globe can be selected. Separately, the backend still contains 5 hand-researched country dossier rows (`IR`, `SY`, `AE`, `SA`, `IQ`), but the shipped UI is primarily driven by the live and seeded measurement datasets above.

## Architecture

- `frontend/` — React 18 + Vite + CesiumJS + Recharts.
- `backend/` — Rust (edition 2024) + Axum + Tokio + `rusqlite` with bundled SQLite.

On startup, the backend creates and seeds the database, starts background fetch loops, and serves the built SPA from `frontend/dist` when present. Satellite orbital elements are cached in memory, while rendered satellite positions are computed fresh on each `/api/satellites` request.

### Satellite catalog persistence

The satellite catalog is the one dataset that cannot be rebuilt on demand: CelesTrak rate-limits repeat downloads and has been observed unreachable at the TCP level for extended periods. It is therefore treated as durable state rather than a cache.

- The canonical catalog is the `satellite_catalog` table, **one row per NORAD catalog ID**. Membership belongs to that table, not to any upstream response.
- A refresh is an *upsert*, never a replacement. Records missing from a response are left untouched, so a truncated or failed fetch cannot shrink the catalog. There is no code path that deletes a satellite.
- Per-object freshness (`epoch`, `last_updated`, `last_seen`) is tracked separately from existence. Degraded data makes an object **stale**, never absent.
- On boot the catalog is loaded from SQLite and served *before* any network call, and a refresh only runs if the persisted catalog is actually due for one — so redeploying repeatedly costs zero upstream requests.
- Merge precedence: the newest element-set **epoch** wins regardless of provider; ties break toward CelesTrak supplemental, then CelesTrak GP, then SatNOGS. CelesTrak and SatNOGS are complementary sources merged into the same catalog, not alternatives.

`GET /api/satellites/status` reports catalog size, the fresh/stale/unpropagatable split, per-provider last success, and the last refresh outcome (`complete` / `degraded` / `failed`).

**Freshness and element age are different questions.** `stale` tracks
`last_updated` — when *we* last accepted a better record. `unpropagatable`
tracks the element set's own `epoch`. They come apart because SatNOGS
republishes long-dead objects: a record accepted a minute ago can carry
orbital data from 1975. Objects whose epoch is older than
`SATELLITE_MAX_PROPAGATION_AGE_DAYS` (default 30) stay in the catalog and are
counted in `total`, but are left out of `/api/satellites` and their orbit
endpoint returns 422 — SGP4 cannot produce a meaningful position from them,
and a confidently-wrong dot is worse than no dot.

This requires persistent storage; see [Persistence](#persistence).

## Data sources

| Source | Used for |
| --- | --- |
| OONI | Technology blocking, messaging reachability, content-category censorship, blocking timelines |
| IODA | Country-level internet outage events |
| Tor Metrics | Relay/bridge users and bridge transport estimates |
| Cloudflare Radar | HTTP-version share and outage annotations |
| RIPEstat | BGP ASN/prefix visibility |
| Internet Society Pulse | Internet Resilience Index |
| V-Dem via Our World in Data | Freedom of expression score and internet-censorship subscores |
| RSF via Our World in Data | Press freedom score |
| PeeringDB | Seeded IXP density lookup |
| TeleGeography | Seeded submarine cable routes and landing points |
| CelesTrak | Satellite catalog and orbital elements |

## Running locally

Prerequisites: Rust stable and Node 18+.

### 1. Backend (`:3001`)

```sh
cd backend
cargo run
```

This creates `mena_ai.db`, loads the seed data, and starts the API immediately. External fetchers then populate and refresh the live datasets in the background.

All configuration is documented in [`.env.example`](.env.example) at the repo
root. Copy it to `.env`; the backend finds it from `backend/` (a `backend/.env`,
if present, takes precedence). Useful optional variables include:

- `DEPLOYMENT_ID`
- `CLOUDFLARE_API_TOKEN`
- `PULSE_API_TOKEN`
- `PEERINGDB_API_KEY` (used by `scripts/gen_ixp_data.mjs`, not by the server)
- `DATABASE_PATH`
- `SEED_DIR`
- `STATIC_DIR`
- `FETCH_INTERVAL_HOURS`
- `PRECISION_FETCH_INTERVAL_HOURS`
- `SATELLITE_CATALOG_REFRESH_HOURS`
- `SATELLITE_STALE_AFTER_HOURS`
- `SATELLITE_GROUP_MIN_RETAIN_RATIO`
- `BIND_ADDR` and `PORT` (default `0.0.0.0:3001`)
- `CORS_ALLOWED_ORIGINS` (unset = same-origin only)

### 2. Frontend (`:5173`)

```sh
cd frontend
npm install
npm run dev
```

Open <http://localhost:5173>. Vite proxies both `/api` and `/health` to `http://localhost:3001` in development (override with `DEV_API_PROXY_TARGET`). The frontend calls the API on its own origin unless `VITE_API_BASE_URL` is set at build time; `VITE_CESIUM_ION_TOKEN` is optional. See `frontend/.env.example`.

### 3. Production build

```sh
cd frontend
npm run build
```

By default, the backend serves `../frontend/dist` when that build output exists.

## API surface

The shipped app mounts these read-only routes:

- `GET /health`
- `GET /api/geo`
- `GET /api/countries`
- `GET /api/countries/:code`
- `GET /api/blocking`
- `GET /api/categories`
- `GET /api/timeline`
- `GET /api/tor-metrics`
- `GET /api/outages`
- `GET /api/censorship-index`
- `GET /api/rankings`
- `GET /api/country-scores`
- `GET /api/cables`
- `GET /api/ixp-stats`
- `GET /api/satellites`
- `GET /api/satellites/status`
- `GET /api/satellites/:norad_id/orbit`
- `GET /api/models`
- `GET /api/signals`
- `GET /api/http-protocol-share`
- `GET /api/bgp-visibility`

Notes:

- `/api/geo` is the whole-world drawable country list used by the globe.
- `/api/countries` is only the small researched-country metadata set.
- `POST /api/evaluate` exists in code but is not mounted in the public read-only app.
- `/api/models` and `/api/signals` are exposed by the backend but are not currently used by the shipped frontend.

## Self-hosting

The production setup is Docker Compose: one app container (the Rust server
plus the built SPA) behind [Caddy](https://caddyserver.com), which terminates
TLS with automatically issued and renewed certificates. Nothing in it is tied
to a hosting provider; any Linux VPS works.

**Requirements**

- A Linux host with Docker Engine and the Compose v2 plugin
  (`docker compose version`).
- A domain whose A/AAAA record points at the host, with ports 80 and 443
  (plus 443/UDP for HTTP/3) reachable from the internet.
- About 4 GB of RAM for the first build, which compiles ~200 crates and SQLite
  from source — add swap on a 2 GB machine. The running app needs far less.

**Configure**

```sh
git clone https://github.com/moumenalaoui/globe.git blackout && cd blackout
cp .env.example .env
```

In `.env`, set `DOMAIN` and `ACME_EMAIL` (required), a `DEPLOYMENT_ID`, and
optionally `CLOUDFLARE_API_TOKEN` and `PULSE_API_TOKEN`. Every variable is
documented in the file. `.env` is gitignored and never enters the image.

**Run**

```sh
docker compose up -d
```

The first run builds the image, which takes several minutes. Then:

- `docker compose ps` shows `app` as `healthy` once the server answers, and
  Caddy starts after it.
- `docker compose logs -f app` shows the boot log; the background fetchers
  fill in the live datasets over the first few minutes.
- `https://<DOMAIN>/health` reports data freshness (200 `ok`, 503 `stale`) —
  point an external uptime monitor at it. `/health/live` is the container's
  own liveness check.

To update: `git pull && docker compose up -d --build`. The database lives on
the `blackout-data` volume and survives rebuilds and restarts;
`docker compose down -v` deletes it. For a consistent backup, stop the app
briefly:

```sh
docker compose stop app
docker run --rm -v blackout-data:/data -v "$PWD":/backup debian:bookworm-slim tar czf /backup/blackout-data.tgz -C /data .
docker compose start app
```

## Deployment notes

### Persistence

The SQLite database is state, not a cache: three fetchers pull bounded
rolling windows and accumulate history locally that upstream will not return
again.

| Table | Window | Fetcher |
| --- | --- | --- |
| `outage_events` | 90 days | `fetchers/ioda.rs` |
| `bgp_prefix_visibility` | 14 days | `fetchers/ripestat.rs` |
| `http_protocol_share` | 14 days | `fetchers/cloudflare_http.rs` |

That is on top of `satellite_catalog`, which cannot be rebuilt on demand at
all. `DATABASE_PATH` must therefore be on persistent storage. In the Docker
setup it is `/data/mena_ai.db` on the named volume `blackout-data`; the boot
log says whether a previous database was found (see [Boot log](#boot-log)).

Runtime cost: `/api/satellites` propagates every matching element set with
SGP4 on each request, polled every 7s per connected browser, and the fetch
loops (6h), precision loops (1h) and satellite refresh (2h) run on timers in
the same process — so the host must stay up, not scale to zero. Two vCPUs are
plenty; `TOKIO_WORKER_THREADS` (default 8 in compose) keeps a slow SQLite
request from stalling the rest.

### Replit (legacy)

`.replit`, `.replitignore` and `replit.nix` remain so the earlier Replit
deployment still builds, but nothing depends on them: the server reads the
same environment variables everywhere, and the Docker build ignores those
files. Replit-specific behaviour is limited to `.replit`'s build command
(a `rustup`-pinned toolchain, because the Nix channel's Cargo is older than
`edition = "2024"`) and an optional snapshot layer in `backend/src/snapshot`.

Because a published Replit app's filesystem is rebuilt on every publish, that
layer restores the database from Replit App Storage at boot and writes it back
every `SNAPSHOT_INTERVAL_MINUTES` (default 180) and on `SIGTERM`. It is inert
unless `SNAPSHOT_KEY` is set, which only `.replit` does. A failed restore never
overwrites a good snapshot: when the download errors no snapshot task is
spawned for that process, and a snapshot whose size collapses below
`SNAPSHOT_MIN_RETAIN_RATIO` of the last good one is refused. In practice the
App Storage sidecar was never reachable from the published container, so the
Replit deployment cold-started on every publish.

### Outbound identity

Every outbound request carries
`blackout-<fetcher>/<version> (+<contact>; id=<DEPLOYMENT_ID>)`, and RIPEstat's
`sourceapp` derives from the same value. The identity is built in
`backend/src/util/http.rs`, every HTTP client in the codebase is constructed
through it, and the boot log prints the result.

This is identification, not rate-limit management. Upstream limits are
enforced either on the credential — Cloudflare Radar, Internet Society Pulse,
PeeringDB — or on the source IP — OONI, IODA, Tor Metrics, RIPEstat, CelesTrak,
SatNOGS and Our World in Data. Several of those are free services run by
volunteers or a small nonprofit, and identifying honestly costs nothing.

### Credentials

All three are free, and all are optional in the sense that the app starts
without them; a `WARNING` block at boot names any that are missing.

| Secret | Gates | Scope |
| --- | --- | --- |
| `CLOUDFLARE_API_TOKEN` | Radar outage annotations and HTTP-version share | `Account` → `Radar` → `Read` |
| `PULSE_API_TOKEN` | Internet Resilience Index | n/a |
| `PEERINGDB_API_KEY` | `scripts/gen_ixp_data.mjs` only, never the server | read-only |

Without the first two, those sources render empty rather than erroring.
`PEERINGDB_API_KEY` matters only when regenerating the committed IXP seed
data: PeeringDB throttles unauthenticated callers hard — an observed
59-minute lockout after three quick requests — though the script works
without one.

### Boot log

A healthy restart prints:

```text
DB at /data/mena_ai.db
DB initialized and seeded.
DB restored: existing database at /data/mena_ai.db (boot #3), 13587 satellite(s) in the persistent catalog
satellites: loaded 13587 satellites from the persistent catalog; catalog age 47m
Listening on 0.0.0.0:3001 (app + API, public read-only)
```

`boot #1` on anything other than a first-ever start means the database was not
carried over — check that `DATABASE_PATH` is on the mounted volume.

## Notes

- The deployment model is public and read-only. There is no auth layer.
- Data freshness is mixed by design: some datasets are periodically fetched, and some are committed seed files.
- Built by Moumen Alaoui at the FAI Hackathon 2026, from [moumenalaoui/BLACKOUT](https://github.com/moumenalaoui/BLACKOUT).
