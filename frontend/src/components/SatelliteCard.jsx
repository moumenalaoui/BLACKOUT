import { BORDER, MONO, MUTED, SIDEBAR, WHITE } from '../theme'
import { SPACE_TRACKING_OPTIONS } from './SatelliteLegend'

// Shared with the legend's row labels for the 5 headline categories; `geo`/
// `other` aren't headline rows there (no dedicated legend row, only reachable
// via "All Satellites") so they get a small local fallback here instead.
const CATEGORY_LABEL = {
  ...Object.fromEntries(SPACE_TRACKING_OPTIONS.map(({ key, label }) => [key, label])),
  geo: 'GEO',
  other: 'Other',
}

// Detail panel for the currently-selected satellite. `satellite` comes
// straight from the live-polled position list — its lat/lon/alt refresh with
// every poll for free, without a fetch of its own. `periodMinutes` comes from
// the separate orbit-path fetch (App.jsx), since only that endpoint computes it.
export default function SatelliteCard({ satellite, periodMinutes, onClose }) {
  if (!satellite) return null

  return (
    <div
      style={{
        position: 'absolute',
        top: 12,
        // Beside SatelliteLegend (left:292, width:196), not on top of it — this
        // used to sit at top:84/left:292, directly overlapping the middle of
        // the legend's row list instead of the empty globe.
        left: 500,
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '10px 12px',
        width: 220,
        zIndex: 5,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
        <div style={{ fontSize: 13, fontWeight: 500, color: WHITE }}>{satellite.name}</div>
        <button
          type="button"
          onClick={onClose}
          style={{ background: 'transparent', border: 'none', color: MUTED, fontSize: 16, lineHeight: 1, cursor: 'pointer' }}
        >
          ×
        </button>
      </div>
      <div style={{ fontFamily: MONO, fontSize: 9, color: MUTED, marginTop: 2, marginBottom: 8 }}>
        NORAD {satellite.norad_id} · {CATEGORY_LABEL[satellite.category] ?? satellite.category}
      </div>
      <dl style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 8px', margin: 0, fontFamily: MONO, fontSize: 10 }}>
        <dt style={{ color: MUTED }}>ALT</dt>
        <dd style={{ margin: 0, color: WHITE }}>{satellite.alt_km.toFixed(1)} km</dd>
        <dt style={{ color: MUTED }}>LAT</dt>
        <dd style={{ margin: 0, color: WHITE }}>{satellite.lat.toFixed(2)}°</dd>
        <dt style={{ color: MUTED }}>LON</dt>
        <dd style={{ margin: 0, color: WHITE }}>{satellite.lon.toFixed(2)}°</dd>
        {periodMinutes != null && (
          <>
            <dt style={{ color: MUTED }}>PERIOD</dt>
            <dd style={{ margin: 0, color: WHITE }}>{periodMinutes.toFixed(1)} min</dd>
          </>
        )}
      </dl>
      <a
        href={`https://www.n2yo.com/satellite/?s=${satellite.norad_id}`}
        target="_blank"
        rel="noopener noreferrer"
        style={{
          display: 'block',
          marginTop: 10,
          textAlign: 'center',
          fontFamily: MONO,
          fontSize: 9,
          letterSpacing: '0.08em',
          color: MUTED,
          border: `1px solid ${BORDER}`,
          padding: '4px 0',
          textDecoration: 'none',
        }}
      >
        TRACK ON N2YO →
      </a>
    </div>
  )
}
