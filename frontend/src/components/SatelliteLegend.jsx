import { BORDER, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

// Matches the default `SATELLITE_GROUPS` categories the backend fetches
// (backend/.env.example) — a v1 subset of the full wishlist (Starlink, GNSS
// constellations, GEO, a generic "other" catch-all), extendable later purely
// via that backend config without any frontend change beyond adding a row here.
export const SATELLITE_CATEGORIES = [
  { key: 'starlink', label: 'Starlink' },
  { key: 'gps', label: 'GPS' },
  { key: 'galileo', label: 'Galileo' },
  { key: 'glonass', label: 'GLONASS' },
  { key: 'beidou', label: 'BeiDou' },
  { key: 'geo', label: 'GEO' },
  { key: 'other', label: 'Other' },
]

export default function SatelliteLegend({ show, onToggleShow, categoryFilters, onToggleCategory, count }) {
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
        padding: '7px 10px',
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        maxWidth: 240,
        zIndex: 5,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: WHITE }}>
          SATELLITES
        </span>
        <button
          type="button"
          onClick={onToggleShow}
          aria-pressed={show}
          style={{
            background: 'transparent',
            border: `1px solid ${show ? HIGHLIGHT : BORDER}`,
            color: show ? HIGHLIGHT : MUTED,
            fontFamily: MONO,
            fontSize: 9,
            letterSpacing: '0.08em',
            padding: '2px 8px',
            cursor: 'pointer',
          }}
        >
          {show ? 'HIDE' : 'SHOW'}
        </button>
        {show && (
          <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>{count} tracked</span>
        )}
      </div>

      {show && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
          {SATELLITE_CATEGORIES.map(({ key, label }) => {
            const on = categoryFilters[key] ?? false
            return (
              <label
                key={key}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 4,
                  fontFamily: MONO,
                  fontSize: 9,
                  letterSpacing: '0.04em',
                  color: on ? WHITE : MUTED,
                  cursor: 'pointer',
                }}
              >
                <input
                  type="checkbox"
                  checked={on}
                  onChange={() => onToggleCategory(key)}
                  style={{ margin: 0 }}
                />
                {label}
              </label>
            )
          })}
        </div>
      )}
    </div>
  )
}
