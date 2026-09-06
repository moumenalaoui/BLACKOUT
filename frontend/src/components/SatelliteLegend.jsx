import { BORDER, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

// Single source of truth for the satellite category taxonomy — the backend
// tags each object with one of these keys (see backend/src/fetchers/
// satellites.rs's DEFAULT_GROUPS) plus two background-only tags ('geo',
// 'other') that aren't given their own row here but are still reachable via
// "All Satellites". Globe.jsx and SatelliteCard.jsx import from here instead
// of keeping their own separate copies of the taxonomy.
export const SPACE_TRACKING_OPTIONS = [
  { key: 'starlink', label: 'Starlink / Comms' },
  { key: 'military', label: 'Military / Intel' },
  { key: 'navigation', label: 'GPS / Navigation' },
  { key: 'earthobs', label: 'Earth Observation' },
  { key: 'stations', label: 'Stations / Telescopes' },
]

// One distinct, thematically-grouped colour per category — deliberately more
// vivid than the app's muted dashboard palette (theme.js), which is tuned for
// text/chrome rather than for telling small dots apart at a glance. `geo`/
// `other` are background-only categories (no dedicated row below, reachable
// only via "All Satellites"), so they get muted, non-competing neutrals.
export const CATEGORY_COLOR_HEX = {
  starlink: '#38bdf8', // sky blue — comms/signal
  navigation: '#fbbf24', // amber/gold — GPS/guidance
  military: '#ef4444', // red — restricted/alert
  earthobs: '#22c55e', // green — Earth/land
  stations: '#a78bfa', // violet — space science
  geo: '#94a3b8', // slate — background-only
  other: '#64748b', // darker slate — background-only
}

function Row({ active, color, label, count, onClick }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        width: '100%',
        background: 'transparent',
        border: 'none',
        padding: '2px 0',
        cursor: 'pointer',
        textAlign: 'left',
      }}
    >
      <span
        aria-hidden="true"
        style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: color,
          flexShrink: 0,
          boxShadow: active ? `0 0 0 2px ${SIDEBAR}, 0 0 0 3px ${color}` : 'none',
        }}
      />
      <span
        style={{
          fontFamily: MONO,
          fontSize: 9,
          letterSpacing: '0.03em',
          color: active ? WHITE : MUTED,
          flex: 1,
          whiteSpace: 'nowrap',
        }}
      >
        {label}
      </span>
      <span style={{ fontFamily: MONO, fontSize: 9, color: active ? WHITE : MUTED }}>
        {count.toLocaleString()}
      </span>
    </button>
  )
}

// Single-select "space tracking" panel: one row is active at a time
// (`selection`), plus a "NONE" row that clears the layer entirely. Every
// row's count comes from the live-polled `counts` (App.jsx's
// spaceTrackingCounts, refreshed from every /api/satellites response's
// total/category_counts) regardless of which row is currently selected/
// fetched — so the whole list stays populated even while viewing one narrow
// category.
export default function SatelliteLegend({ selection, onSelect, counts }) {
  return (
    <div
      style={{
        position: 'absolute',
        top: 12,
        // GlobalRanking ("Most Censored Countries") occupies the whole left
        // edge at left:12, width:264 — sit just to its right, not on top of it.
        left: 292,
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '6px 8px',
        display: 'flex',
        flexDirection: 'column',
        gap: 2,
        width: 196,
        zIndex: 5,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: 1,
        }}
      >
        <span style={{ fontFamily: MONO, fontSize: 9, letterSpacing: '0.1em', color: WHITE }}>
          SPACE TRACKING
        </span>
        <button
          type="button"
          onClick={() => onSelect('none')}
          aria-pressed={selection === 'none'}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 3,
            background: 'transparent',
            border: `1px solid ${selection === 'none' ? HIGHLIGHT : BORDER}`,
            color: selection === 'none' ? HIGHLIGHT : MUTED,
            fontFamily: MONO,
            fontSize: 8,
            letterSpacing: '0.08em',
            padding: '1px 4px',
            cursor: 'pointer',
          }}
        >
          NONE ✕
        </button>
      </div>

      <Row
        active={selection === 'all'}
        color={WHITE}
        label="All Satellites"
        count={counts.total ?? 0}
        onClick={() => onSelect('all')}
      />
      {SPACE_TRACKING_OPTIONS.map(({ key, label }) => (
        <Row
          key={key}
          active={selection === key}
          color={CATEGORY_COLOR_HEX[key]}
          label={label}
          count={counts[key] ?? 0}
          onClick={() => onSelect(key)}
        />
      ))}
    </div>
  )
}
