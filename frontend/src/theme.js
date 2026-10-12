// Layered slate surface system — a blue-black base with three elevation levels
// reads as "engineered" where a flat #000 reads as empty. BLACK is retained for
// the Cesium globe scene, which stays pure black underneath the slate chrome
// that floats over it.
export const BLACK         = '#000000'
export const BASE          = '#080b11'
export const SIDEBAR       = '#0d121a'
export const RAISED        = '#141c28'
export const BORDER        = '#1b2531'
export const BORDER_STRONG = '#2b3a4b'
export const WHITE         = '#e6e9ef'
export const MUTED         = '#8c95a3'

// HUD / interactive accent. Additive to the existing crimson=alert and
// gold=selection language: cyan marks live interactive surfaces (layer toggles,
// the globe reticle and graticule) and never competes with a blocking-status
// color.
export const CYAN          = '#37c0e6'
export const CYAN_DIM      = '#1c4a5a'

export const US_EXPOSURE = '#a0aec0'
export const CN_EXPOSURE = '#c9822b'
export const LOCAL       = '#6c9a5b'
export const CRIMSON     = '#c8102e'
export const HIGHLIGHT   = '#d6b36a'
export const AMBER       = '#d97706'
export const DIM         = '#3d4f6b'

export const MONO        = '"IBM Plex Mono", "Fira Mono", monospace'
export const SANS        = '"Inter", system-ui, sans-serif'

// Type scale, in px, calibrated against the ranking list: Inter at 11 for
// names and running text, Plex Mono at 10 for figures and labels — one step
// above the original UI, whose 8–9px text read too small. Mono glyphs are
// much wider than Inter's, so mono reads visibly bigger than Inter at the same
// size — mono text therefore tops out at `label` (numeric readouts in the
// header aside), and `body` is for Inter. Pick a step by role; never
// hand-pick an off-scale size.
export const TYPE = {
  tick:     9, // chart axis ticks, footnotes, source tags
  status:   9.5, // bottom status bar: between footnote and control size
  label:   10, // mono: uppercase labels, codes, figures, controls
  body:    11, // Inter: names, list entries, running text
  title:   13, // country / satellite name, wordmark
  display: 15, // close glyph
}

// Tier colors are pinned to the stack-dependency palette rather than an
// arbitrary ramp — blue reads as "safe/trusted" which is wrong for a tier
// defined by foreign compute dependency.
export const TIER_COLORS = {
  COMPREHENSIVELY_SANCTIONED: CRIMSON,
  POST_SANCTIONS_LAG:         CN_EXPOSURE,
  US_COMPUTE_STACK:           US_EXPOSURE,
  RESOURCE_CONSTRAINED:       LOCAL,
}
