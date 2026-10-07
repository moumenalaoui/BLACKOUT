import { useEffect, useState } from 'react'
import BgpVisibilityChart from './BgpVisibilityChart'
import CategoryBreakdown from './CategoryBreakdown'
import FreedomRank from './FreedomRank'
import GlobalIndices from './GlobalIndices'
import Http3ShareChart from './Http3ShareChart'
import MessagingStatus from './MessagingStatus'
import OutageTimeline from './OutageTimeline'
import ResilienceIndex from './ResilienceIndex'
import IxpBadge from './IxpBadge'
import TimelineChart from './TimelineChart'
import TorChart from './TorChart'
import {
  BLOCKING_REGISTRY,
  BLOCKING_STATUS_COLOR,
  BLOCKING_STATUS_LABEL,
  GROUP_LABELS,
  TECH_BLURBS,
  TECH_LABELS,
  hasTimeline,
} from '../lib/blockingRegistry'
import { FREEDOM_KEY, NETWORK_KEY, TOR_CHART_KEY } from '../lib/sectionKeys'
import CollapsibleSection from './CollapsibleSection'
import { BORDER, BORDER_STRONG, MONO, MUTED, SANS, SIDEBAR, TYPE, WHITE } from '../theme'

const ALL_TECHNOLOGIES = Object.values(BLOCKING_REGISTRY).flat()

// A visual "theme" is one level above the small per-widget MONO headers
// (BLOCKING STATUS, MESSAGING APPS, etc.) — it groups related signals (access
// blocking, censorship, network/protocol, resilience) behind a bolder label
// and a real divider line, so unrelated signals don't read as one continuous
// blob when they only had an 18px gap between them.
//
// paddingBottom matters as much as paddingTop here: with only paddingTop, the
// divider line sits flush against whatever the *previous* section's last
// widget happens to render (a quiet footnote vs. a dense row of pill badges),
// so the same 16px gap reads as "loose" after one widget and "cramped" after
// another. Padding both sides guarantees a fixed quiet zone around the line
// itself, independent of what content sits on either side of it.
//
// `keyEntries` adds a collapsed "WHAT THESE ARE" key under the title, held
// close to it (8px, not the 18px between widgets) so it reads as part of the
// heading rather than as another widget.
function ThemeSection({ title, first, keyEntries, children }) {
  return (
    <section
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 18,
        paddingTop: first ? 0 : 16,
        paddingBottom: 16,
        borderTop: first ? 'none' : `1px solid ${BORDER_STRONG}`,
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {/* Inter semibold, title case: the one level that isn't mono caps, so
            a section title reads as a heading rather than as another of the
            per-widget labels beneath it (which differed only by colour). */}
        <div style={{ fontFamily: SANS, fontSize: TYPE.title, fontWeight: 600, color: WHITE }}>{title}</div>
        {keyEntries?.length > 0 && <SectionKey entries={keyEntries} />}
      </div>
      {children}
    </section>
  )
}

// What a section's terms mean, collapsed by default. A tap target rather
// than a hover tooltip, so it works in the phone bottom sheet too.
function SectionKey({ entries }) {
  return (
    <CollapsibleSection title="WHAT THESE ARE">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {entries.map(({ term, blurb }) => (
          <div key={term}>
            <div style={{ fontFamily: MONO, fontSize: TYPE.label, color: WHITE }}>{term}</div>
            <div style={{ fontFamily: SANS, fontSize: TYPE.body, color: MUTED, lineHeight: 1.4 }}>{blurb}</div>
          </div>
        ))}
      </div>
    </CollapsibleSection>
  )
}

function BlockSegments({ filledCount, color }) {
  return (
    <div style={{ display: 'flex', gap: 1, flexShrink: 0 }}>
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} style={{ width: 6, height: 6, background: i < filledCount ? color : BORDER }} />
      ))}
    </div>
  )
}

// Days of history a timeline needs before it counts as a signal on its own.
//
// `> 0` was too low to mean anything. OONI publishes the occasional isolated
// day for a technology it barely covers in a country, and one such row was
// enough to promote the technology, resurrect its whole group, and add a
// group heading — Iran had exactly one such day against 400-990 days
// for every technology actually tracked there. The resulting chart is a
// single point, which draws no line at all, under a "1 days · peak ..."
// footnote.
//
// A week is the smallest window where a sparkline shows a shape rather than a
// dot, and it sits far below the real technologies' hundreds of days, so it
// separates noise from signal without hiding anything genuine.
const MIN_TIMELINE_DAYS = 7

// A row is worth showing only if it carries an actual signal: either the
// point-in-time classification resolved to something other than
// inconclusive/no-data, or there's enough historical timeline behind it. A
// blank "INCONCLUSIVE / 0" row tells a reader nothing and just adds noise.
function isMeaningful(row, timelineRows) {
  const hasPointSignal = !!row && row.measurement_count > 0 && row.status !== 'INCONCLUSIVE'
  return hasPointSignal || hasEnoughTimeline(timelineRows)
}

function hasEnoughTimeline(timelineRows) {
  return (timelineRows?.length ?? 0) >= MIN_TIMELINE_DAYS
}

// "Still fetching" and "there is genuinely nothing here" used to render
// identically — as nothing at all — so a country with complete data looked
// broken for the whole of a cold start. That window is minutes, not
// milliseconds: OONI's timeline phase is the slowest thing in the app (ten
// 2-D aggregations over every country since 2024-01-01, up to ~18 MB each),
// and until it lands every blocking section is empty.
//
// This is the same distinction the pulse fetcher already insists on in its
// own boot warning — an empty panel has to say whether it is empty because
// the data has not arrived or because the source does not cover this country.
function SectionState({ loading, emptyLabel }) {
  return (
    <p style={{ fontFamily: MONO, fontSize: TYPE.label, letterSpacing: '0.06em', color: MUTED }}>
      {loading ? 'LOADING\u2026' : emptyLabel}
    </p>
  )
}

function BlockingGroupList({ groups, blockingByTech, timelineByTech, countryCode, showGroupLabel = true }) {
  return (
    <>
      {groups.map(({ group, techs }, i) => (
        <div
          key={group}
          style={{
            marginBottom: 10,
            paddingTop: i === 0 ? 0 : 8,
            borderTop: i === 0 ? 'none' : `1px solid ${BORDER}`,
          }}
        >
          {showGroupLabel && (
            <p
              style={{
                fontFamily: MONO,
                fontSize: TYPE.label,
                letterSpacing: '0.06em',
                textTransform: 'uppercase',
                color: MUTED,
                marginBottom: 2,
              }}
            >
              {GROUP_LABELS[group]}
            </p>
          )}
          {techs.map((tech) => (
            <BlockingTechRow
              key={tech}
              tech={tech}
              row={blockingByTech[tech]}
              countryCode={countryCode}
              timelineRows={timelineByTech[tech]}
            />
          ))}
        </div>
      ))}
      {/* The row's bar and trailing number are otherwise unlabelled. */}
      <p style={{ fontFamily: MONO, fontSize: TYPE.tick, color: MUTED, letterSpacing: '0.05em' }}>
        Bar: share of OONI tests showing anomalies (possible blocking) · number: tests run
      </p>
    </>
  )
}

function BlockingTechRow({ tech, row, countryCode, timelineRows }) {
  const status = row?.status ?? 'NO_DATA'
  const count = row?.measurement_count ?? 0
  const anomalyRate = row?.anomaly_rate ?? 0
  const color = BLOCKING_STATUS_COLOR[status] ?? BORDER
  const filledCount = Math.max(0, Math.min(8, Math.round(anomalyRate * 8)))
  const showTimeline = hasTimeline(countryCode, tech) && hasEnoughTimeline(timelineRows)

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 22 }}>
        <span style={{ fontFamily: MONO, fontSize: TYPE.label, color: WHITE, width: 96, flexShrink: 0 }}>
          {TECH_LABELS[tech] ?? tech}
        </span>
        <BlockSegments filledCount={filledCount} color={color} />
        <span style={{ fontFamily: MONO, fontSize: TYPE.label, color, width: 86, flexShrink: 0 }}>
          {BLOCKING_STATUS_LABEL[status]}
        </span>
        <span style={{ fontFamily: MONO, fontSize: TYPE.label, color: MUTED }}>{count}</span>
      </div>
      {showTimeline && (
        <div style={{ padding: '6px 0 6px 0' }}>
          <TimelineChart rows={timelineRows} />
        </div>
      )}
    </div>
  )
}

export default function CountrySidebar({ country, layer, ixpStats, onClose, fill = false }) {
  const [blockingRows, setBlockingRows] = useState([])
  const [blockingLoading, setBlockingLoading] = useState(true)
  const [timelineByTech, setTimelineByTech] = useState({})
  const [timelineLoading, setTimelineLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    setBlockingRows([])
    setBlockingLoading(true)

    fetch(`/api/blocking?country=${country.country_code}`)
      .then((response) => (response.ok ? response.json() : []))
      .then((rows) => {
        if (!cancelled) setBlockingRows(rows)
      })
      .catch(() => {
        if (!cancelled) setBlockingRows([])
      })
      .finally(() => {
        if (!cancelled) setBlockingLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [country.country_code])

  useEffect(() => {
    let cancelled = false
    setTimelineByTech({})
    setTimelineLoading(true)

    const promoted = ALL_TECHNOLOGIES.filter((tech) => hasTimeline(country.country_code, tech))
    Promise.all(
      promoted.map((tech) =>
        fetch(`/api/timeline?country=${country.country_code}&technology=${tech}`)
          .then((response) => (response.ok ? response.json() : []))
          .then((rows) => [tech, rows])
          .catch(() => [tech, []]),
      ),
    ).then((entries) => {
      if (!cancelled) {
        setTimelineByTech(Object.fromEntries(entries))
        setTimelineLoading(false)
      }
    })

    return () => {
      cancelled = true
    }
  }, [country.country_code])

  const blockingByTech = Object.fromEntries(blockingRows.map((row) => [row.technology, row]))
  const groupsForLayer = layer === 'AI_ACCESS' || layer === 'CIRCUMVENTION'
    ? [layer]
    : ['AI_ACCESS', 'CIRCUMVENTION']

  const visibleGroups = groupsForLayer
    .map((group) => ({
      group,
      techs: BLOCKING_REGISTRY[group].filter((tech) => isMeaningful(blockingByTech[tech], timelineByTech[tech])),
    }))
    .filter(({ techs }) => techs.length > 0)

  const aiAccessGroups = visibleGroups.filter(({ group }) => group === 'AI_ACCESS')
  const circumventionGroups = visibleGroups.filter(({ group }) => group === 'CIRCUMVENTION')
  // Only the blocked tools actually listed for this country, so the key never
  // explains a row the reader can't see; then the Tor chart's terms.
  const circumventionKey = [
    ...circumventionGroups
      .flatMap(({ techs }) => techs)
      .filter((tech) => TECH_BLURBS[tech])
      .map((tech) => ({ term: TECH_LABELS[tech] ?? tech, blurb: TECH_BLURBS[tech] })),
    ...TOR_CHART_KEY,
  ]

  // A row is only considered settled once BOTH fetches are in: `isMeaningful`
  // promotes a technology on either a point-in-time classification or a
  // timeline, so a group that looks empty with only one of the two loaded may
  // still fill in.
  const blockingPending = blockingLoading || timelineLoading
  // Which sections the current layer filter admits at all. Without this, a
  // CIRCUMVENTION-only view would report "no coverage" for AI access, when in
  // fact it was simply not asked for.
  const showAiAccess = groupsForLayer.includes('AI_ACCESS')
  const showCircumvention = groupsForLayer.includes('CIRCUMVENTION')

  return (
    <div
      style={{
        // `fill`: the phone bottom sheet, which sets its own width.
        width: fill ? '100%' : 360,
        minWidth: fill ? 0 : 360,
        height: '100%',
        overflowY: 'auto',
        background: SIDEBAR,
        borderLeft: fill ? 'none' : `1px solid ${BORDER}`,
      }}
    >
      <div style={{ padding: '16px 20px', borderBottom: `1px solid ${BORDER}` }}>
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <h2 style={{ fontSize: TYPE.display, fontWeight: 600, color: WHITE }}>{country.country_name}</h2>
            <span style={{ fontFamily: MONO, fontSize: TYPE.label, letterSpacing: '0.05em', color: MUTED }}>
              {country.country_code}
            </span>
          </div>
          <button
            onClick={onClose}
            style={{ background: 'transparent', border: 'none', color: MUTED, fontSize: TYPE.display, lineHeight: 1, cursor: 'pointer' }}
          >
            ×
          </button>
        </div>
        <FreedomRank countryCode={country.country_code} />
      </div>

      <div style={{ padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 0 }}>
        {/* Ordered for a policy reader: the headline freedom scores first,
            then what's blocked (messaging, AI, content, circumvention tools),
            and the technical network measurements last. */}
        <ThemeSection title="Freedom & Resilience" first keyEntries={FREEDOM_KEY}>
          <GlobalIndices countryCode={country.country_code} />

          <ResilienceIndex countryCode={country.country_code} />
        </ThemeSection>

        <ThemeSection title="Messaging">
          <MessagingStatus countryCode={country.country_code} />
        </ThemeSection>

        {showAiAccess && (
          <ThemeSection title="AI Access">
            <div>
              <div style={{ fontFamily: MONO, fontSize: TYPE.label, letterSpacing: '0.06em', color: MUTED, marginBottom: 8 }}>
                BLOCKING STATUS
              </div>
              {aiAccessGroups.length > 0 ? (
                <BlockingGroupList
                  groups={aiAccessGroups}
                  blockingByTech={blockingByTech}
                  timelineByTech={timelineByTech}
                  countryCode={country.country_code}
                  showGroupLabel={false}
                />
              ) : (
                <SectionState loading={blockingPending} emptyLabel="NO OONI COVERAGE" />
              )}
            </div>
          </ThemeSection>
        )}

        <ThemeSection title="Censorship">
          <CategoryBreakdown countryCode={country.country_code} />
        </ThemeSection>

        <ThemeSection title="Circumvention" keyEntries={circumventionKey}>
          {showCircumvention && (
            <div>
              <div style={{ fontFamily: MONO, fontSize: TYPE.label, letterSpacing: '0.06em', color: MUTED, marginBottom: 8 }}>
                BLOCKING STATUS
              </div>
              {circumventionGroups.length > 0 ? (
                <BlockingGroupList
                  groups={circumventionGroups}
                  blockingByTech={blockingByTech}
                  timelineByTech={timelineByTech}
                  countryCode={country.country_code}
                  // One group here, so a label would only restate the
                  // section title.
                  showGroupLabel={false}
                />
              ) : (
                <SectionState loading={blockingPending} emptyLabel="NO OONI COVERAGE" />
              )}
            </div>
          )}

          {/* Tor relay/bridge usage. Rendered from its own /api/tor-metrics data
              (it returns null when empty) rather than gated on a "meaningful" tor
              blocking row, so it surfaces for every country that has Tor data —
              not just those with a point-in-time blocking classification. */}
          <TorChart countryCode={country.country_code} />
        </ThemeSection>

        <ThemeSection title="Network & Protocol" keyEntries={NETWORK_KEY}>
          <OutageTimeline countryCode={country.country_code} />

          <Http3ShareChart countryCode={country.country_code} />

          <BgpVisibilityChart countryCode={country.country_code} />

          <IxpBadge entry={ixpStats} />
        </ThemeSection>
      </div>
    </div>
  )
}
