use serde::{Deserialize, Serialize};

/// Daily HTTP/1.x vs HTTP/2 vs HTTP/3 (QUIC) traffic share for one country,
/// from Cloudflare Radar. Nullable percentages: Cloudflare omits a version
/// entirely at ~0% share rather than sending an explicit zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProtocolShare {
    pub id: String,
    pub country_code: String,
    pub date: String,
    pub http1_pct: Option<f64>,
    pub http2_pct: Option<f64>,
    pub http3_pct: Option<f64>,
    pub source: String,
}
