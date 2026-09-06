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
  - blocking status across AI access, circumvention tools, and privacy-focused operating systems
  - messaging app reachability
  - censored content categories
  - 90-day outage history
  - Tor relay and bridge usage
  - HTTP/1.x, HTTP/2, and HTTP/3 traffic share
  - BGP visibility
  - Internet Exchange Point density
  - Starlink restriction status
  - Internet Society Pulse resilience scores
  - V-Dem and RSF freedom scores
- Shows a global "most censored countries" ranking panel and a footer with source links, link state, and data age.

Any drawable country on the globe can be selected. Separately, the backend still contains 5 hand-researched country dossier rows (`IR`, `SY`, `AE`, `SA`, `IQ`), but the shipped UI is primarily driven by the live and seeded measurement datasets above.

## Architecture

- `frontend/` — React 18 + Vite + CesiumJS + Recharts.
- `backend/` — Rust (edition 2024) + Axum + Tokio + `rusqlite` with bundled SQLite.

On startup, the backend creates and seeds the database, starts background fetch loops, and serves the built SPA from `frontend/dist` when present. Satellite orbital elements are cached in memory, while rendered satellite positions are computed fresh on each `/api/satellites` request.

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
| Manual seed data | Starlink country restrictions |

## Running locally

Prerequisites: Rust stable and Node 18+.

### 1. Backend (`:3001`)

```sh
cd backend
cargo run
```

This creates `mena_ai.db`, loads the seed data, and starts the API immediately. External fetchers then populate and refresh the live datasets in the background.

Useful optional env vars are documented in `backend/.env.example`, including:

- `CLOUDFLARE_API_TOKEN`
- `PULSE_API_TOKEN`
- `DATABASE_PATH`
- `SEED_DIR`
- `STATIC_DIR`
- `FETCH_INTERVAL_HOURS`
- `PRECISION_FETCH_INTERVAL_HOURS`
- `SATELLITE_CATALOG_REFRESH_HOURS`
- `PORT`

### 2. Frontend (`:5173`)

```sh
cd frontend
npm install
npm run dev
```

Open <http://localhost:5173>. Vite proxies both `/api` and `/health` to `http://localhost:3001` in development. `VITE_CESIUM_ION_TOKEN` is optional; the current globe setup does not require it.

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
- `GET /api/starlink-status`
- `GET /api/cables`
- `GET /api/ixp-stats`
- `GET /api/satellites`
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

## Notes

- The deployment model is public and read-only. There is no auth layer.
- Data freshness is mixed by design: some datasets are periodically fetched, some are committed seed files, and Starlink status is manually maintained.
- Built by Moumen Alaoui at the FAI Hackathon 2026.
