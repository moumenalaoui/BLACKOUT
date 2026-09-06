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
export const SOURCES = [
  { id: 'OONI', label: 'OONI' },
  { id: 'IODA', label: 'IODA' },
  { id: 'TOR', label: 'Tor' },
  { id: 'CLOUDFLARE', label: 'Cloudflare' },
  { id: 'RIPESTAT', label: 'RIPEstat' },
  { id: 'ISOC_PULSE', label: 'ISOC Pulse' },
  { id: 'V_DEM', label: 'V-Dem' },
  { id: 'RSF', label: 'RSF' },
  { id: 'PEERINGDB', label: 'PeeringDB' },
  { id: 'TELEGEOGRAPHY', label: 'TeleGeography' },
  { id: 'CELESTRAK', label: 'CelesTrak' },
]
