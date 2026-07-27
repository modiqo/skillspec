//! Environment-variable extraction.
//!
//! Finds `$NAME` and `${NAME}` references and marks the ones whose names are
//! credential-shaped. The mark rides on the effect target rather than being a
//! separate finding, because a credential read produces a grant and the grant is
//! what a reviewer decides on.

use std::collections::BTreeSet;

/// Name fragments that make a variable credential-shaped.
const CREDENTIAL_FRAGMENTS: &[&str] = &[
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "APIKEY",
    "API_KEY",
    "ACCESS_KEY",
    "PRIVATE_KEY",
    "CREDENTIAL",
    "SESSION_KEY",
    "AUTH",
];

/// Whole names that are credential-shaped without matching a fragment.
const CREDENTIAL_NAMES: &[&str] = &["AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN", "NPM_TOKEN"];

/// Name prefixes whose variables carry credentials by convention.
const CREDENTIAL_PREFIXES: &[&str] = &["AWS_", "GH_", "GITHUB_", "ANTHROPIC_", "OPENAI_"];

/// Variables that match a credential prefix but carry no secret.
const PREFIX_EXCEPTIONS: &[&str] = &[
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AWS_PROFILE",
    "AWS_PAGER",
    "GITHUB_REPOSITORY",
    "GITHUB_WORKSPACE",
    "GITHUB_ACTOR",
    "GITHUB_REF",
    "GITHUB_SHA",
    "GITHUB_RUN_ID",
];

/// Every distinct variable referenced in `text`, in stable order.
pub fn referenced_variables(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        let rest = &text[index + 1..];
        let (name, consumed) = if let Some(inner) = rest.strip_prefix('{') {
            match inner.find('}') {
                Some(end) => (&inner[..end], end + 2),
                None => {
                    index += 1;
                    continue;
                }
            }
        } else {
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .unwrap_or(rest.len());
            (&rest[..end], end)
        };

        if is_variable_name(name) {
            found.insert(name.to_owned());
        }
        index += 1 + consumed.max(1);
    }
    found
}

/// Whether a variable name is credential-shaped.
pub fn is_credential_like(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    if PREFIX_EXCEPTIONS.contains(&upper.as_str()) {
        return false;
    }
    if CREDENTIAL_NAMES.contains(&upper.as_str()) {
        return true;
    }
    if CREDENTIAL_FRAGMENTS
        .iter()
        .any(|fragment| upper.contains(fragment))
    {
        return true;
    }
    CREDENTIAL_PREFIXES
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn is_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !name.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{is_credential_like, referenced_variables};

    fn vars(text: &str) -> Vec<String> {
        referenced_variables(text).into_iter().collect()
    }

    #[test]
    fn bare_and_braced_references_are_both_found() {
        assert_eq!(vars("echo $HOME and ${PATH}"), ["HOME", "PATH"]);
    }

    #[test]
    fn references_are_deduplicated_and_ordered() {
        assert_eq!(vars("$B $A $B"), ["A", "B"]);
    }

    #[test]
    fn adjacent_punctuation_ends_a_name() {
        assert_eq!(vars("curl \"$ENDPOINT/collect\""), ["ENDPOINT"]);
        assert_eq!(vars("${OWNER}/${REPO}"), ["OWNER", "REPO"]);
    }

    #[test]
    fn positional_and_special_parameters_are_ignored() {
        assert!(vars("$1 $@ $? $$").is_empty());
    }

    #[test]
    fn unterminated_braces_do_not_capture() {
        assert!(vars("${UNCLOSED").is_empty());
    }

    #[test]
    fn credential_shaped_names_are_recognized() {
        for name in [
            "GITHUB_TOKEN",
            "NPM_TOKEN",
            "MY_API_KEY",
            "SERVICE_SECRET",
            "DB_PASSWORD",
            "AWS_SECRET_ACCESS_KEY",
            "ANTHROPIC_API_KEY",
            "AUTH_HEADER",
        ] {
            assert!(is_credential_like(name), "{name}");
        }
    }

    #[test]
    fn ordinary_names_are_not_credential_shaped() {
        for name in ["HOME", "PATH", "EDITOR", "PROJECT_ROOT", "LANG"] {
            assert!(!is_credential_like(name), "{name}");
        }
    }

    #[test]
    fn benign_variables_under_a_credential_prefix_are_excepted() {
        // AWS_ and GITHUB_ carry plenty of harmless CI variables; marking them
        // all credential-like would make the mark meaningless.
        for name in [
            "AWS_REGION",
            "AWS_PROFILE",
            "GITHUB_REPOSITORY",
            "GITHUB_SHA",
        ] {
            assert!(!is_credential_like(name), "{name}");
        }
        assert!(is_credential_like("AWS_SECRET_ACCESS_KEY"));
        assert!(is_credential_like("GITHUB_TOKEN"));
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert!(is_credential_like("github_token"));
        assert!(is_credential_like("my_api_key"));
    }
}
