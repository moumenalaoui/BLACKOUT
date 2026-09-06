import { BORDER, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

// Static/decorative infrastructure context, not a censorship signal — a
// single binary toggle (unlike SatelliteLegend's multi-select), default off
// so it doesn't compete with the always-relevant layers on first load.
//
// Unpositioned on purpose — App.jsx renders this inside a shared, centered
// bottom row alongside IndexLegend, so the pair centers as one group
// regardless of either panel's content-driven width. Pixel-offset placements
// tried earlier (squeezed next to OutageFeed, then stacked under SPACE
// TRACKING) each broke at some combination of viewport width and sidebar
// state; a flex-centered group has no such dependency.
export default function CableLegend({ show, onToggle, routeCount, landingCount }) {
  return (
    <div
      // The route/landing-point count + attribution used to sit inline as a
      // caption. Same info is still here, just on hover, so the panel is
      // only as wide as its label + toggle — the same footprint as every
      // other compact legend (IndexLegend, SatelliteLegend).
      title={`${routeCount} cables · ${landingCount} landing points · via TeleGeography`}
      style={{
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '7px 10px',
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        flexShrink: 0,
        whiteSpace: 'nowrap',
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
    </div>
  )
}
