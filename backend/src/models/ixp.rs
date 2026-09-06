use serde::{Deserialize, Serialize};

/// Per-country Internet Exchange Point density, generated once (by
/// `backend/scripts/gen_ixp_data.mjs`) from PeeringDB's public directory —
/// the structural counterpart to the BGP-visibility signal: a country routed
/// through very few domestic exchange points has to send nearly all its
/// traffic through a handful of international gateways, which is what makes a
/// full national shutdown fast and cheap. Absence of a row means zero known
/// IXPs, not "unknown" — that's itself the strongest possible reading of this
/// signal, so callers should treat a missing country as `ixp_count: 0`,
/// not hide it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxpStats {
    pub country_code: String,
    pub ixp_count: i64,
    pub total_net_count: i64,
    pub largest_ixp_name: String,
    pub largest_ixp_net_count: i64,
    pub generated_at: String,
}
