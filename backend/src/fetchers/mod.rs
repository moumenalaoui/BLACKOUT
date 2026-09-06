pub mod cloudflare;
pub mod cloudflare_http;
pub mod indices;
pub mod ioda;
pub mod ooni;
pub mod pulse;
pub mod ripestat;
pub mod satellites;
pub mod tor_metrics;

// Every fetcher now covers the whole globe by grouping/tagging country
// server-side (OONI via probe_cc, Tor via the CSV country column, Cloudflare via
// each annotation's `locations`) and filtering against `known_codes`, or — for
// IODA, cloudflare_http, and ripestat — by reading `codes_by_priority`
// directly. There is no shared per-country iterator here anymore.
