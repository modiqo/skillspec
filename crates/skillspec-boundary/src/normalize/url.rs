//! URL normalization.
//!
//! Reduces a URL written in package source to the host a grant would name, plus
//! enough detail for review: whether the host is a bare IP, whether it carries
//! non-ASCII characters, and how much of it was determinable.
//!
//! Interpolation in the **host** makes the target dynamic and therefore
//! ungrantable. Interpolation only in the path leaves the host intact, because
//! the grant is over the host and the path does not change it.

use crate::effect::TargetResolution;

/// A URL reduced to the parts a grant and a review need.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedUrl {
    pub host: String,
    pub scheme: Option<String>,
    pub port: Option<u16>,
    pub ip_literal: bool,
    pub non_ascii: bool,
    pub resolution: TargetResolution,
}

/// Normalize a URL. Returns `None` when `raw` has no recoverable host.
pub fn normalize(raw: &str) -> Option<NormalizedUrl> {
    // Prose wraps URLs in quotes, parentheses, and sentence punctuation.
    let trimmed = raw
        .trim()
        .trim_start_matches(['"', '\'', '`', '(', '[', '<'])
        .trim_end_matches(['"', '\'', '`', ',', ';', ')', ']', '>', '.']);
    if trimmed.is_empty() {
        return None;
    }

    let (scheme, rest) = match trimmed.split_once("://") {
        Some((scheme, rest)) if is_scheme(scheme) => (Some(scheme.to_ascii_lowercase()), rest),
        _ => (None, trimmed),
    };

    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim();
    if authority.is_empty() {
        return None;
    }

    // Drop any userinfo; credentials in a URL are not part of the grant.
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, tail)| tail);
    let (host, port) = split_port(authority);
    if host.is_empty() {
        return None;
    }

    let host_interpolated = contains_interpolation(host);
    let path_interpolated = contains_interpolation(rest) && !host_interpolated;

    let resolution = if host_interpolated {
        TargetResolution::Dynamic
    } else if path_interpolated {
        TargetResolution::Templated
    } else {
        TargetResolution::Literal
    };

    let normalized_host = if host_interpolated {
        host.to_owned()
    } else {
        host.trim_end_matches('.').to_ascii_lowercase()
    };

    Some(NormalizedUrl {
        ip_literal: is_ip_literal(&normalized_host),
        non_ascii: !normalized_host.is_ascii(),
        host: normalized_host,
        scheme,
        port,
        resolution,
    })
}

fn is_scheme(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

/// Split `host:port`, leaving bracketed IPv6 literals intact.
fn split_port(authority: &str) -> (&str, Option<u16>) {
    if let Some(end) = authority.find(']') {
        let (host, tail) = authority.split_at(end + 1);
        let port = tail.strip_prefix(':').and_then(|port| port.parse().ok());
        return (host, port);
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) => {
            (host, port.parse().ok())
        }
        _ => (authority, None),
    }
}

fn contains_interpolation(value: &str) -> bool {
    value.contains('$') || value.contains("{{") || value.contains("%s")
}

fn is_ip_literal(host: &str) -> bool {
    if host.starts_with('[') && host.ends_with(']') {
        return true;
    }
    let octets = host.split('.').collect::<Vec<_>>();
    octets.len() == 4
        && octets
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::effect::TargetResolution;

    #[test]
    fn plain_https_url_resolves_to_its_host() {
        let url = normalize("https://api.github.com/user").unwrap();
        assert_eq!(url.host, "api.github.com");
        assert_eq!(url.scheme.as_deref(), Some("https"));
        assert_eq!(url.resolution, TargetResolution::Literal);
        assert!(!url.ip_literal);
        assert!(!url.non_ascii);
    }

    #[test]
    fn host_case_and_trailing_dot_are_normalized() {
        assert_eq!(
            normalize("https://API.GitHub.COM./x").unwrap().host,
            "api.github.com"
        );
    }

    #[test]
    fn interpolation_in_the_path_keeps_the_host_grantable() {
        let url = normalize("https://api.github.com/repos/$OWNER/$REPO").unwrap();
        assert_eq!(url.host, "api.github.com");
        assert_eq!(url.resolution, TargetResolution::Templated);
        assert!(url.resolution.is_grantable());
    }

    #[test]
    fn interpolation_in_the_host_makes_the_target_ungrantable() {
        let url = normalize("https://$ENDPOINT/collect").unwrap();
        assert_eq!(url.resolution, TargetResolution::Dynamic);
        assert!(!url.resolution.is_grantable());
    }

    #[test]
    fn userinfo_is_not_part_of_the_host() {
        let url = normalize("https://user:token@internal.example.com/path").unwrap();
        assert_eq!(url.host, "internal.example.com");
    }

    #[test]
    fn ports_are_captured_separately() {
        let url = normalize("http://localhost:8080/health").unwrap();
        assert_eq!(url.host, "localhost");
        assert_eq!(url.port, Some(8080));
    }

    #[test]
    fn ipv4_and_ipv6_literals_are_marked() {
        assert!(normalize("http://192.168.1.10/x").unwrap().ip_literal);
        assert!(normalize("http://[2001:db8::1]:9000/x").unwrap().ip_literal);
        assert!(!normalize("https://example.com/x").unwrap().ip_literal);
    }

    #[test]
    fn non_ascii_hosts_are_marked_for_review() {
        // A homoglyph host produces a grant, so the flag rides on the target
        // rather than being a separate detector.
        let url = normalize("https://gіthub.com/x").unwrap();
        assert!(url.non_ascii);
    }

    #[test]
    fn trailing_punctuation_from_prose_is_stripped() {
        assert_eq!(
            normalize("https://example.com/docs.").unwrap().host,
            "example.com"
        );
        assert_eq!(
            normalize("(https://example.com/docs)").unwrap().host,
            "example.com"
        );
    }

    #[test]
    fn schemeless_hosts_still_normalize() {
        let url = normalize("api.example.com/v1").unwrap();
        assert_eq!(url.host, "api.example.com");
        assert_eq!(url.scheme, None);
    }

    #[test]
    fn empty_and_hostless_input_yields_nothing() {
        assert!(normalize("").is_none());
        assert!(normalize("https://").is_none());
        assert!(normalize("   ").is_none());
    }
}
