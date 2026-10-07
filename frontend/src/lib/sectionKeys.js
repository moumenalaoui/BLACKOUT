// Copy for the collapsed "WHAT THESE ARE" keys in the country sidebar, one
// list per section that needs one. Only the sections whose terms are jargon
// to a policy reader get a key — Messaging, AI Access and Censorship read
// plainly without one. Each `term` matches the label its widget shows, so a
// reader can find it; wording follows methodology/METHODOLOGY-value.md.
// The Circumvention key also lists the blocked tools shown for the country —
// see TECH_BLURBS in blockingRegistry.js.

export const FREEDOM_KEY = [
  {
    term: 'V-Dem — Freedom of Expression',
    blurb:
      'An expert assessment by the V-Dem Institute. Its sub-scores rate government internet filtering, internet shutdowns and censorship effort. Higher means freer.',
  },
  {
    term: 'RSF — Press Freedom',
    blurb: 'Reporters Without Borders’ press freedom score, from the edition before its 2022 change in method. Higher means freer.',
  },
  {
    term: 'Pulse Resilience Index',
    blurb:
      'The Internet Society’s measure of how well a country’s internet withstands disruption, built from infrastructure, performance, security and market readiness. Higher means more resilient.',
  },
]

// The Tor usage chart's terms, after the blocked-tool entries.
export const TOR_CHART_KEY = [
  { term: 'Relay users', blurb: 'Estimated number of people connecting to Tor directly.' },
  {
    term: 'Bridge users',
    blurb: 'Estimated number of people connecting through unlisted Tor entry points (“bridges”), used where Tor itself is blocked.',
  },
  {
    term: 'obfs4 · Snowflake · webtunnel',
    blurb:
      'Types of bridge, each disguising Tor traffic differently: as random noise, as a video call relayed through volunteers’ browsers, or as ordinary web browsing.',
  },
]

export const NETWORK_KEY = [
  { term: 'Internet outages', blurb: 'Drops in national connectivity detected by IODA over the last 90 days, by severity.' },
  {
    term: 'HTTP/3 traffic share',
    blurb:
      'Share of web traffic using HTTP/3, the newest web protocol, which runs over QUIC. Some censors block QUIC, so a sudden fall can signal interference.',
  },
  {
    term: 'BGP prefix visibility',
    blurb: 'Share of the country’s networks that the rest of the internet can still reach. A collapse means networks have gone dark, as in a shutdown.',
  },
  {
    term: 'Internet exchange points',
    blurb:
      'Local hubs where networks swap traffic. Few or none means traffic leaves through a handful of international gateways, which makes a shutdown easier to impose.',
  },
]
