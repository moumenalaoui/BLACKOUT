import { AMBER, BORDER, CRIMSON, DIM, LOCAL } from '../theme'

export const BLOCKING_REGISTRY = {
  AI_ACCESS: ['openai.com', 'claude.ai', 'deepseek', 'huggingface'],
  CIRCUMVENTION: ['tor', 'torproject', 'signal', 'i2p', 'psiphon', 'torsf'],
}

export const GROUP_LABELS = {
  AI_ACCESS: 'AI Access',
  CIRCUMVENTION: 'Circumvention',
}

// Reader-facing names for the technology keys above, which are OONI's own
// identifiers (`torsf`, `i2p`) and mean nothing to a policy reader. A key
// that is a web_connectivity probe against a URL is labelled with that
// domain, so it reads as "does this website load" rather than "does this tool
// work" — `signal` in particular is signal.org, while the Signal app itself
// is measured under Messaging (see backend/src/fetchers/ooni.rs).
export const TECH_LABELS = {
  'openai.com': 'openai.com',
  'claude.ai': 'claude.ai',
  deepseek: 'deepseek.com',
  huggingface: 'huggingface.co',
  tor: 'Tor',
  torsf: 'Tor (Snowflake)',
  psiphon: 'Psiphon',
  torproject: 'torproject.org',
  i2p: 'geti2p.net',
  signal: 'signal.org',
}

// One line per circumvention technology, for the key in the sidebar's
// Circumvention section.
export const TECH_BLURBS = {
  tor: 'Whether a connection to the Tor anonymity network can be made.',
  torsf: 'Tor routed through volunteer browser proxies, built for places that block Tor itself.',
  psiphon: 'Whether the Psiphon circumvention app can connect.',
  torproject: 'Whether the Tor Project’s website, where Tor is downloaded, loads.',
  i2p: 'Whether the website of I2P, an anonymity network similar to Tor, loads.',
  signal: 'Whether Signal’s website loads. Whether the Signal app itself works is shown under Messaging.',
}

export const BLOCKING_STATUS_COLOR = {
  CONFIRMED_BLOCKED: CRIMSON,
  LIKELY_BLOCKED: AMBER,
  ACCESSIBLE: LOCAL,
  INCONCLUSIVE: DIM,
  NO_DATA: BORDER,
}

// Ranks status severity so a country/layer with mixed technology statuses is
// represented by its single worst one.
export const BLOCKING_SEVERITY = {
  CONFIRMED_BLOCKED: 3,
  LIKELY_BLOCKED: 2,
  ACCESSIBLE: 1,
  INCONCLUSIVE: 0,
}

const LAYERS = ['AI_ACCESS', 'CIRCUMVENTION']

function worseOf(current, next) {
  if (current == null) return next
  return (BLOCKING_SEVERITY[next] ?? -1) > (BLOCKING_SEVERITY[current] ?? -1) ? next : current
}

/// Collapses per-technology blocking rows into a per-country worst status,
/// overall and per layer.
///
/// Single pass over `rows`. The previous shape took a country roster and ran
/// four full `rows.filter()` sweeps per country, i.e. O(countries x rows) —
/// which is fine for five countries and roughly half a million comparisons for
/// the whole world. Keyed by whatever appears in the data rather than by a
/// supplied roster, since every caller already treats a missing key as NO_DATA.
export function buildBlockingMap(rows) {
  const map = {}
  for (const row of rows) {
    const entry = (map[row.country_code] ??= { ALL: null, AI_ACCESS: null, CIRCUMVENTION: null })
    entry.ALL = worseOf(entry.ALL, row.status)
    if (LAYERS.includes(row.category)) {
      entry[row.category] = worseOf(entry[row.category], row.status)
    }
  }
  for (const entry of Object.values(map)) {
    for (const key of ['ALL', 'AI_ACCESS', 'CIRCUMVENTION']) entry[key] ??= 'NO_DATA'
  }
  return map
}

export const BLOCKING_STATUS_LABEL = {
  CONFIRMED_BLOCKED: 'BLOCKED',
  LIKELY_BLOCKED: 'LIKELY',
  ACCESSIBLE: 'ACCESSIBLE',
  INCONCLUSIVE: 'INCONCLUSIVE',
  NO_DATA: 'NO DATA',
}

// Technologies the backend backfills a daily blocking timeline for — the only
// technologies worth rendering a sparkline for. Kept in sync with TIMELINE_TECHS
// in backend/src/fetchers/ooni.rs. The backend now backfills these for *every*
// country (grouped server-side by probe_cc), so there is no country allowlist:
// a timeline exists for any country that has measurements. A chart therefore
// renders whenever there are rows and hides itself when there aren't, rather
// than being gated on a hardcoded five.
const TIMELINE_TECHNOLOGIES = [
  'tor',
  'torproject',
  'signal',
  'i2p',
  'psiphon',
  'torsf',
  'openai.com',
  'claude.ai',
  'deepseek',
  'huggingface',
]

export function hasTimeline(_countryCode, technology) {
  return TIMELINE_TECHNOLOGIES.includes(technology)
}
