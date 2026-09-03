use crate::error::ProtoError;
use crate::messages::{NegotiateRequest, NegotiateResponse, SemVer};
fn parse_semver(s: &str) -> Option<SemVer> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(SemVer { major, minor, patch })
}
fn cmp(a: &SemVer, b: &SemVer) -> std::cmp::Ordering {
    a.major.cmp(&b.major).then(a.minor.cmp(&b.minor)).then(a.patch.cmp(&b.patch))
}
pub fn supports_version(req: &str, min: &str, max: &str) -> bool {
    let Some(r) = parse_semver(req) else { return false };
    let Some(lo) = parse_semver(min) else { return false };
    let Some(hi) = parse_semver(max) else { return false };
    cmp(&r, &lo) != std::cmp::Ordering::Less && cmp(&r, &hi) != std::cmp::Ordering::Greater
}
pub fn negotiate(req: &NegotiateRequest, min_supported: &str, max_supported: &str) -> Result<NegotiateResponse, ProtoError> {
    let min = parse_semver(min_supported).ok_or_else(|| ProtoError::Validation("invalid min_supported".into()))?;
    let max = parse_semver(max_supported).ok_or_else(|| ProtoError::Validation("invalid max_supported".into()))?;
    let ver = parse_semver(&req.protocol_version).ok_or_else(|| ProtoError::Validation("invalid protocol_version".into()))?;
    let accepted = cmp(&ver, &min) != std::cmp::Ordering::Less && cmp(&ver, &max) != std::cmp::Ordering::Greater;
    if accepted {
        Ok(NegotiateResponse { accepted: true, error: None, min_supported: min_supported.to_string(), max_supported: max_supported.to_string() })
    } else {
        Err(ProtoError::VersionUnsupported(format!("{} not in {} - {}", req.protocol_version, min_supported, max_supported)))
    }
}
