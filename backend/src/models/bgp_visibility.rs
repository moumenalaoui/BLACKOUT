use serde::{Deserialize, Serialize};

/// Daily BGP prefix/ASN visibility for one country, from RIPEstat: how much
/// of the country's *registered* address space is *currently routed*
/// (visible in the global routing table) versus merely allocated on paper.
/// Counts are `f64`, not integers: RIPEstat averages across multiple RIS
/// route-collector snapshots within its resolution window, so a busy
/// country's count is frequently fractional. Deliberately no precomputed
/// ratio field — see `bgp_prefix_visibility`'s table comment in
/// db/schema.rs for why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpVisibility {
    pub id: String,
    pub country_code: String,
    pub date: String,
    pub registered_asns: Option<f64>,
    pub routed_asns: Option<f64>,
    pub registered_v4_prefixes: Option<f64>,
    pub routed_v4_prefixes: Option<f64>,
    pub registered_v6_prefixes: Option<f64>,
    pub routed_v6_prefixes: Option<f64>,
    pub source: String,
}
