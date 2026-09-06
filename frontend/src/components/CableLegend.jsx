import { BORDER, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

// Static/decorative infrastructure context, not a censorship signal — a
// single binary toggle (unlike SatelliteLegend's multi-select), default off
// so it doesn't compete with the always-relevant layers on first load.
export default function CableLegend({ show, onToggle, routeCount, landingCount }) {
  return (
    <div
      style={{
        position: 'absolute',
        // Bottom-right is otherwise only used by the conditional, transient
        // status-message toast at bottom:16 — sitting above it here avoids
        // collision even in the rare case both are visible at once. (Not
        // top-right: OutageFeed already occupies that corner.)
        bottom: 60,
        right: 12,
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '7px 10px',
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        zIndex: 5,
      }}
    >
      <span style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: WHITE }}>
        SUBMARINE CABLES
      </span>

      <button
        type="button"
        onClick={onToggle}
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

      <span style={{ fontFamily: MONO, fontSize: 8, color: MUTED }}>
        {routeCount} cables · {landingCount} landing points · via TeleGeography
      </span>
    </div>
  )
}
