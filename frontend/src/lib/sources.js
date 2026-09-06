// The open-source feeds the backend ingests (see backend/src/fetchers/*).
// Surfaced in the UI chrome purely as provenance — this list is display-only,
// drives no fetching, and must stay a faithful reflection of what the backend
// actually pulls. The backend owns all ingestion.
//
//   Telemetry / measurement:  OONI, IODA, Tor Metrics, Cloudflare, RIPEstat
//   Freedom / resilience:     V-Dem, RSF, ISOC Pulse
//                             (V-Dem/RSF arrive via Our World in Data's
//                              republished grapher CSVs — see indices.rs)
//   Structural / static:      PeeringDB (IXP density), TeleGeography
//                             (submarine cables), CelesTrak (satellites)
//
// Deliberately excludes Starlink country status: that dataset is hand-curated
// against multiple third-party roundups rather than pulled from one
// attributable feed (see StarlinkBadge.jsx / data/seed/starlink_status.json),
// so it cites its sources inline instead of claiming a spot in this list.
// `url` is each source's own public homepage/dashboard, not the raw fetcher
// endpoint (backend/src/fetchers/*.rs hits API routes under these same
// domains — see there for the exact ones) — a reader clicking through wants a
// human-readable landing page, not a bare JSON/CSV response. V-Dem and RSF
// point at the specific Our World in Data grapher pages actually pulled from
// (indices.rs), since that — not v-dem.net or rsf.org directly — is where
// this app's numbers come from.
export const SOURCES = [
  { id: 'OONI', label: 'OONI', url: 'https://ooni.org' },
  { id: 'IODA', label: 'IODA', url: 'https://ioda.inetintel.cc.gatech.edu' },
  { id: 'TOR', label: 'Tor', url: 'https://metrics.torproject.org' },
  { id: 'CLOUDFLARE', label: 'Cloudflare', url: 'https://radar.cloudflare.com' },
  { id: 'RIPESTAT', label: 'RIPEstat', url: 'https://stat.ripe.net' },
  { id: 'ISOC_PULSE', label: 'ISOC Pulse', url: 'https://pulse.internetsociety.org' },
  { id: 'V_DEM', label: 'V-Dem', url: 'https://ourworldindata.org/grapher/freedom-of-expression-index' },
  { id: 'RSF', label: 'RSF', url: 'https://ourworldindata.org/grapher/press-freedom-index-rsf' },
  { id: 'PEERINGDB', label: 'PeeringDB', url: 'https://www.peeringdb.com' },
  { id: 'TELEGEOGRAPHY', label: 'TeleGeography', url: 'https://www.submarinecablemap.com' },
  { id: 'CELESTRAK', label: 'CelesTrak', url: 'https://celestrak.org' },
]
