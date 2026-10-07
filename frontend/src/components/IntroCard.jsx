import { BORDER, MONO, MUTED, SANS, SIDEBAR, TYPE, WHITE } from '../theme'

// First-visit explainer at the globe's top-left, the corner every dock panel
// leaves empty on load. The header's tagline says what BLACKOUT measures, but
// nothing said what to *do* — and the phone header drops the tagline
// entirely. App decides when it shows (nothing selected, no panel open) and
// retires it for the session once a country is selected or it's closed.
export default function IntroCard({ narrow = false, onClose }) {
  return (
    <aside
      aria-label="About this map"
      style={{
        position: 'absolute',
        top: narrow ? 8 : 12,
        left: narrow ? 8 : 12,
        maxWidth: narrow ? 'calc(100% - 16px)' : 280,
        display: 'flex',
        alignItems: 'flex-start',
        gap: 10,
        padding: '10px 12px',
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        zIndex: 5,
      }}
    >
      <p style={{ fontFamily: SANS, fontSize: TYPE.body, lineHeight: 1.45, color: WHITE }}>
        Use this tool to see where the internet is blocked, censored or shut down around the world.{' '}
        <span style={{ color: MUTED }}>Select a country on the globe, or search for one above, to get started.</span>
      </p>
      <button
        type="button"
        onClick={onClose}
        aria-label="Close"
        style={{ background: 'transparent', border: 'none', color: MUTED, fontFamily: MONO, fontSize: TYPE.label, cursor: 'pointer', padding: 0 }}
      >
        ✕
      </button>
    </aside>
  )
}
