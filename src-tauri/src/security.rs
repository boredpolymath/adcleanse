//! Shared security primitives: strict URL allowlisting, IPC input validation,
//! and CSV formula-injection neutralization.
//!
//! All outbound-URL and webview-navigation checks MUST go through
//! [`is_allowed_https_url`]. Raw `str::starts_with` prefix checks are unsafe:
//! `https://graph.facebook.com.evil.tld/` and `https://graph.facebook.com@evil.tld/`
//! both start with `https://graph.facebook.com`.

use crate::error::{AdCleanseError, Result};
use reqwest::Url;

/// Upper bound on IDs/names accepted over IPC.
pub const MAX_IDENTIFIER_LEN: usize = 128;
/// Upper bound on batch operations accepted over IPC.
pub const MAX_BATCH_SIZE: usize = 500;
/// Upper bound on a user-defined rule pattern.
pub const MAX_RULE_PATTERN_LEN: usize = 256;
/// Upper bound on any remote response body we are willing to buffer.
pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
/// Upper bound on topics/partners accepted from a single remote payload.
pub const MAX_PARSED_ITEMS: usize = 5_000;

/// An allowlist rule: exact host match plus an optional path prefix.
pub struct AllowedEndpoint {
    pub host: &'static str,
    /// Path prefix that must match on a segment boundary ("/" matches everything).
    pub path_prefix: &'static str,
}

/// Returns true only if `raw` is an `https` URL with no userinfo, no explicit
/// non-default port, an exact host match, and a path matching the rule's prefix
/// on a path-segment boundary.
pub fn is_allowed_https_url(raw: &str, allowlist: &[AllowedEndpoint]) -> bool {
    let url = match Url::parse(raw) {
        Ok(u) => u,
        Err(_) => return false,
    };
    if url.scheme() != "https" {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if url.port().is_some() {
        return false;
    }
    let host = match url.host_str() {
        Some(h) => h.to_ascii_lowercase(),
        None => return false,
    };
    let path = url.path();
    allowlist.iter().any(|rule| {
        if host != rule.host {
            return false;
        }
        let prefix = rule.path_prefix;
        if prefix == "/" {
            return true;
        }
        path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('.'))
    })
}

/// Validates an opaque identifier received over IPC before it is ever used in a
/// URL path, SQL parameter, or log line. Allows `[A-Za-z0-9_.:-]` only.
pub fn validate_identifier(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_LEN {
        return Err(AdCleanseError::InvalidInput(format!(
            "{field} must be 1-{MAX_IDENTIFIER_LEN} characters"
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(AdCleanseError::InvalidInput(format!(
            "{field} contains disallowed characters"
        )));
    }
    Ok(())
}

/// Validates free-text display labels received over IPC (topic/company names).
pub fn validate_label(value: &str, field: &str) -> Result<()> {
    if value.chars().count() > MAX_IDENTIFIER_LEN * 2 {
        return Err(AdCleanseError::InvalidInput(format!(
            "{field} exceeds maximum length"
        )));
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(AdCleanseError::InvalidInput(format!(
            "{field} contains control characters"
        )));
    }
    Ok(())
}

/// Produces a safely quoted CSV cell. Embedded quotes are doubled (RFC 4180) and
/// cells beginning with spreadsheet formula triggers are prefixed with `'` so
/// attacker-influenced advertiser/topic names cannot execute as formulas
/// (CWE-1236) when the export is opened in Excel/Sheets/Numbers.
pub fn csv_cell(value: &str) -> String {
    let needs_neutralizing = value
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    let mut out = String::with_capacity(value.len() + 4);
    out.push('"');
    if needs_neutralizing {
        out.push('\'');
    }
    for c in value.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Stable, version-independent 64-bit FNV-1a hash used to derive deterministic
/// IDs for remote objects that arrive without one. (`DefaultHasher` is explicitly
/// not stable across Rust releases, which would silently break snapshot diffing.)
pub fn stable_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &[AllowedEndpoint] = &[
        AllowedEndpoint {
            host: "graph.facebook.com",
            path_prefix: "/",
        },
        AllowedEndpoint {
            host: "www.facebook.com",
            path_prefix: "/login",
        },
    ];

    #[test]
    fn url_allowlist_rejects_prefix_confusion_attacks() {
        assert!(is_allowed_https_url("https://graph.facebook.com/v19.0/me", RULES));
        assert!(is_allowed_https_url("https://www.facebook.com/login.php", RULES));
        assert!(is_allowed_https_url("https://www.facebook.com/login/device", RULES));

        assert!(!is_allowed_https_url("https://graph.facebook.com.evil.tld/", RULES));
        assert!(!is_allowed_https_url("https://graph.facebook.com@evil.tld/", RULES));
        assert!(!is_allowed_https_url("https://user:pw@graph.facebook.com/", RULES));
        assert!(!is_allowed_https_url("https://graph.facebook.com:8443/", RULES));
        assert!(!is_allowed_https_url("http://graph.facebook.com/", RULES));
        assert!(!is_allowed_https_url("https://www.facebook.com/loginevil", RULES));
        assert!(!is_allowed_https_url("https://www.facebook.com/settings", RULES));
        assert!(!is_allowed_https_url("javascript:alert(1)", RULES));
        assert!(!is_allowed_https_url("not a url", RULES));
    }

    #[test]
    fn identifier_validation() {
        assert!(validate_identifier("top_123-abc.v2:x", "id").is_ok());
        assert!(validate_identifier("", "id").is_err());
        assert!(validate_identifier("../../etc/passwd", "id").is_err());
        assert!(validate_identifier("a/b", "id").is_err());
        assert!(validate_identifier("a?b=c", "id").is_err());
        assert!(validate_identifier(&"a".repeat(MAX_IDENTIFIER_LEN + 1), "id").is_err());
        assert!(validate_label("Health & Wellness", "name").is_ok());
        assert!(validate_label("bad\nname", "name").is_err());
    }

    #[test]
    fn csv_cells_are_escaped_and_neutralized() {
        assert_eq!(csv_cell("plain"), "\"plain\"");
        assert_eq!(csv_cell("a\"b"), "\"a\"\"b\"");
        assert_eq!(
            csv_cell("=HYPERLINK(\"http://x\")"),
            "\"'=HYPERLINK(\"\"http://x\"\")\""
        );
        assert_eq!(csv_cell("@SUM(1)"), "\"'@SUM(1)\"");
    }

    #[test]
    fn stable_hash_is_deterministic() {
        assert_eq!(stable_hash("abc"), 0xe71f_a219_0541_574b);
    }
}
