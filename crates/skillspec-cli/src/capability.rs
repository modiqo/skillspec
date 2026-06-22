use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct AddOptions {
    pub id: String,
    pub domain: String,
    pub kind: String,
    pub command: Option<String>,
    pub adapter: Option<String>,
    pub script: Option<String>,
    pub provides: Vec<String>,
    pub aliases: Vec<String>,
    pub priority: Option<u8>,
    pub preferred_for: Vec<String>,
    pub avoid_for: Vec<String>,
    pub ties: Vec<String>,
    pub auth_env: Vec<String>,
    pub external_service: bool,
    pub may_cost_money: bool,
    pub evidence_command: Vec<String>,
    pub suggested_skill_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub capability: String,
    pub domain: Option<String>,
    pub local_only: bool,
    pub preferred_seed: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PreferOptions {
    pub id: String,
    pub domain: Option<String>,
    pub for_capability: String,
    pub priority: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySeed {
    pub id: String,
    pub domain: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "CapabilityRank::is_empty")]
    pub rank: CapabilityRank,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<CapabilityEvidence>,
    #[serde(default, skip_serializing_if = "CapabilityAuth::is_empty")]
    pub auth: CapabilityAuth,
    #[serde(default, skip_serializing_if = "CapabilityRisk::is_empty")]
    pub risk: CapabilityRisk,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion: Option<CapabilityPromotion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<CapabilityVerification>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRank {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_for: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub avoid_for: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tie_breakers: BTreeMap<String, String>,
}

impl CapabilityRank {
    fn is_empty(&self) -> bool {
        self.default_priority.is_none()
            && self.preferred_for.is_empty()
            && self.avoid_for.is_empty()
            && self.tie_breakers.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEvidence {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityAuth {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
}

impl CapabilityAuth {
    fn is_empty(&self) -> bool {
        self.env.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRisk {
    #[serde(default, skip_serializing_if = "is_false")]
    pub external_service: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub may_cost_money: bool,
}

impl CapabilityRisk {
    fn is_empty(&self) -> bool {
        !self.external_service && !self.may_cost_money
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityPromotion {
    pub suggested_skill_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityVerification {
    pub status: VerificationStatus,
    pub verified_at_unix: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<VerificationOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Failed,
    Unverified,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationOutcome {
    pub check: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StoreReport {
    pub seed_store: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeedWriteReport {
    pub seed_store: String,
    pub id: String,
    pub domain: String,
    pub path: String,
    pub status: &'static str,
    pub seed: CapabilitySeed,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeedListReport {
    pub seed_store: String,
    pub domain: Option<String>,
    pub seeds: Vec<SeedSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeedSummary {
    pub id: String,
    pub domain: String,
    pub kind: String,
    pub path: String,
    pub provides: Vec<String>,
    pub verification_status: VerificationStatus,
    pub risk: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeedInspectReport {
    pub seed_store: String,
    pub path: String,
    pub seed: CapabilitySeed,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchReport {
    pub domain: Option<String>,
    pub capability: String,
    pub seed_store: String,
    pub selected: Option<String>,
    pub selection_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_policy: Option<AskPolicy>,
    pub candidates: Vec<SearchCandidate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AskPolicy {
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchCandidate {
    pub id: String,
    pub domain: String,
    pub path: String,
    pub score: i64,
    pub status: &'static str,
    pub reasons: Vec<String>,
    pub risk: Vec<String>,
    pub required_gates: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerifyReport {
    pub seed_store: String,
    pub id: String,
    pub domain: String,
    pub path: String,
    pub status: VerificationStatus,
    pub outcomes: Vec<VerificationOutcome>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RemoveReport {
    pub seed_store: String,
    pub id: String,
    pub path: String,
    pub removed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanReport {
    pub seed_store: String,
    pub proposals: Vec<CapabilitySeed>,
    pub message: String,
}

#[derive(Clone, Debug)]
struct LoadedSeed {
    path: PathBuf,
    seed: CapabilitySeed,
}

pub fn store() -> Result<StoreReport> {
    Ok(StoreReport {
        seed_store: seed_store()?.display().to_string(),
    })
}

pub fn add(options: AddOptions) -> Result<SeedWriteReport> {
    validate_id("capability id", &options.id)?;
    validate_id("domain", &options.domain)?;
    if options.provides.is_empty() {
        return Err(Error::MissingField { field: "provides" });
    }
    for capability in &options.provides {
        validate_id("provides", capability)?;
    }
    let priority = validate_priority(options.priority)?;
    let tie_breakers = parse_ties(&options.ties)?;
    let evidence = options
        .evidence_command
        .iter()
        .map(|command| CapabilityEvidence {
            source: "cli_help".to_owned(),
            command: Some(command.clone()),
            detail: None,
        })
        .collect::<Vec<_>>();
    let promotion = options
        .suggested_skill_id
        .map(|suggested_skill_id| CapabilityPromotion { suggested_skill_id });
    let seed = CapabilitySeed {
        id: options.id,
        domain: options.domain,
        kind: options.kind,
        command: options.command,
        adapter: options.adapter,
        script: options.script,
        provides: options.provides,
        aliases: options.aliases,
        rank: CapabilityRank {
            default_priority: priority,
            preferred_for: options.preferred_for,
            avoid_for: options.avoid_for,
            tie_breakers,
        },
        evidence,
        auth: CapabilityAuth {
            env: options.auth_env,
        },
        risk: CapabilityRisk {
            external_service: options.external_service,
            may_cost_money: options.may_cost_money,
        },
        promotion,
        verification: None,
    };
    write_seed(seed, "written")
}

pub fn prefer(options: PreferOptions) -> Result<SeedWriteReport> {
    validate_id("capability id", &options.id)?;
    validate_id("for", &options.for_capability)?;
    let priority = validate_priority(options.priority)?;
    let loaded = find_seed(&options.id, options.domain.as_deref())?;
    let mut seed = loaded.seed;
    if !seed
        .rank
        .preferred_for
        .iter()
        .any(|capability| canonical(capability) == canonical(&options.for_capability))
    {
        seed.rank.preferred_for.push(options.for_capability);
    }
    if let Some(priority) = priority {
        seed.rank.default_priority = Some(priority);
    }
    write_seed_at(seed, &loaded.path, "updated")
}

pub fn list(domain: Option<&str>) -> Result<SeedListReport> {
    if let Some(domain) = domain {
        validate_id("domain", domain)?;
    }
    let store = seed_store()?;
    let mut seeds = load_seeds(domain)?;
    seeds.sort_by(|left, right| {
        left.seed
            .domain
            .cmp(&right.seed.domain)
            .then(left.seed.id.cmp(&right.seed.id))
    });
    Ok(SeedListReport {
        seed_store: store.display().to_string(),
        domain: domain.map(str::to_owned),
        seeds: seeds
            .into_iter()
            .map(|loaded| {
                let verification_status = verification_status(&loaded.seed);
                let risk = risk_flags(&loaded.seed);
                SeedSummary {
                    id: loaded.seed.id,
                    domain: loaded.seed.domain,
                    kind: loaded.seed.kind,
                    path: loaded.path.display().to_string(),
                    provides: loaded.seed.provides,
                    verification_status,
                    risk,
                }
            })
            .collect(),
    })
}

pub fn inspect(id: &str, domain: Option<&str>) -> Result<SeedInspectReport> {
    validate_id("capability id", id)?;
    if let Some(domain) = domain {
        validate_id("domain", domain)?;
    }
    let store = seed_store()?;
    let loaded = find_seed(id, domain)?;
    Ok(SeedInspectReport {
        seed_store: store.display().to_string(),
        path: loaded.path.display().to_string(),
        seed: loaded.seed,
    })
}

pub fn search(options: SearchOptions) -> Result<SearchReport> {
    validate_id("capability", &options.capability)?;
    if let Some(domain) = &options.domain {
        validate_id("domain", domain)?;
    }
    if let Some(seed_id) = &options.preferred_seed {
        validate_id("preferred seed", seed_id)?;
    }
    let store = seed_store()?;
    let mut candidates = load_seeds(options.domain.as_deref())?
        .into_iter()
        .filter_map(|loaded| score_candidate(&options, loaded))
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.id.cmp(&right.id))
            .then(left.domain.cmp(&right.domain))
    });

    let ask_policy = close_candidate_ask_policy(&candidates);
    let selected = if ask_policy.is_none() {
        candidates.first().map(|candidate| candidate.id.clone())
    } else {
        None
    };

    Ok(SearchReport {
        domain: options.domain,
        capability: options.capability,
        seed_store: store.display().to_string(),
        selected,
        selection_policy: "highest_score_after_constraints",
        ask_policy,
        candidates,
    })
}

pub fn verify(id: &str, domain: Option<&str>) -> Result<VerifyReport> {
    validate_id("capability id", id)?;
    if let Some(domain) = domain {
        validate_id("domain", domain)?;
    }
    let store = seed_store()?;
    let loaded = find_seed(id, domain)?;
    let mut seed = loaded.seed;
    let mut outcomes = Vec::new();

    if let Some(command) = &seed.command {
        outcomes.push(verify_path_lookup(command));
    }
    for evidence in &seed.evidence {
        if let Some(command) = &evidence.command {
            outcomes.push(verify_evidence_command(command));
        }
    }
    if outcomes.is_empty() {
        outcomes.push(VerificationOutcome {
            check: "evidence".to_owned(),
            ok: false,
            message: "no verification evidence declared".to_owned(),
        });
    }

    let status = if outcomes.iter().any(|outcome| outcome.ok) {
        VerificationStatus::Verified
    } else {
        VerificationStatus::Failed
    };
    seed.verification = Some(CapabilityVerification {
        status: status.clone(),
        verified_at_unix: now_unix(),
        outcomes: outcomes.clone(),
    });
    let report = write_seed_at(seed, &loaded.path, "verified")?;
    Ok(VerifyReport {
        seed_store: store.display().to_string(),
        id: report.id,
        domain: report.domain,
        path: report.path,
        status,
        outcomes,
    })
}

pub fn remove(id: &str, domain: Option<&str>) -> Result<RemoveReport> {
    validate_id("capability id", id)?;
    if let Some(domain) = domain {
        validate_id("domain", domain)?;
    }
    let store = seed_store()?;
    let loaded = find_seed(id, domain)?;
    fs::remove_file(&loaded.path).map_err(|source| Error::Write {
        path: loaded.path.clone(),
        source,
    })?;
    Ok(RemoveReport {
        seed_store: store.display().to_string(),
        id: loaded.seed.id,
        path: loaded.path.display().to_string(),
        removed: true,
    })
}

pub fn scan() -> Result<ScanReport> {
    let store = seed_store()?;
    Ok(ScanReport {
        seed_store: store.display().to_string(),
        proposals: Vec::new(),
        message: "no scan providers are configured; use `skillspec capability add` to seed an installed CLI or adapter".to_owned(),
    })
}

fn write_seed(seed: CapabilitySeed, status: &'static str) -> Result<SeedWriteReport> {
    let path = seed_path(&seed.domain, &seed.id)?;
    write_seed_at(seed, &path, status)
}

fn write_seed_at(
    seed: CapabilitySeed,
    path: &Path,
    status: &'static str,
) -> Result<SeedWriteReport> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let content = serde_yaml::to_string(&seed).map_err(|source| Error::RenderYaml {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(SeedWriteReport {
        seed_store: seed_store()?.display().to_string(),
        id: seed.id.clone(),
        domain: seed.domain.clone(),
        path: path.display().to_string(),
        status,
        seed,
    })
}

fn load_seeds(domain: Option<&str>) -> Result<Vec<LoadedSeed>> {
    let store = seed_store()?;
    if !store.exists() {
        return Ok(Vec::new());
    }
    let root = match domain {
        Some(domain) => store.join(domain),
        None => store.clone(),
    };
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_seed_files(&root, &mut files)?;
    files
        .into_iter()
        .map(|path| {
            let content = fs::read_to_string(&path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })?;
            let seed = serde_yaml::from_str::<CapabilitySeed>(&content).map_err(|source| {
                Error::ParseYaml {
                    path: path.clone(),
                    source,
                }
            })?;
            validate_seed(&seed)?;
            if let Some(domain) = domain {
                if seed.domain != domain {
                    return Err(Error::InvalidInput {
                        message: format!(
                            "seed {} is stored under domain {domain:?} but declares domain {:?}",
                            path.display(),
                            seed.domain
                        ),
                    });
                }
            }
            Ok(LoadedSeed { path, seed })
        })
        .collect()
}

fn collect_seed_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })? {
        let path = entry
            .map_err(|source| Error::Read {
                path: dir.to_path_buf(),
                source,
            })?
            .path();
        if path.is_dir() {
            collect_seed_files(&path, files)?;
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        ) {
            files.push(path);
        }
    }
    Ok(())
}

fn find_seed(id: &str, domain: Option<&str>) -> Result<LoadedSeed> {
    let matches = load_seeds(domain)?
        .into_iter()
        .filter(|loaded| loaded.seed.id == id)
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(Error::InvalidInput {
            message: format!("unknown capability seed {id:?}"),
        }),
        1 => Ok(matches.into_iter().next().expect("one match")),
        _ => Err(Error::InvalidInput {
            message: format!("capability seed {id:?} exists in multiple domains; pass --domain"),
        }),
    }
}

fn score_candidate(options: &SearchOptions, loaded: LoadedSeed) -> Option<SearchCandidate> {
    let status = verification_status(&loaded.seed);
    if status == VerificationStatus::Failed {
        return None;
    }

    let requested = canonical(&options.capability);
    let mut score = 0_i64;
    let mut reasons = Vec::new();
    let direct_match = loaded
        .seed
        .provides
        .iter()
        .any(|capability| canonical(capability) == requested);
    let alias_match = loaded
        .seed
        .aliases
        .iter()
        .any(|alias| canonical(alias) == requested || text_matches(alias, &options.capability));
    let evidence_match = loaded.seed.evidence.iter().any(|evidence| {
        evidence
            .command
            .as_deref()
            .is_some_and(|command| text_matches(command, &options.capability))
            || evidence
                .detail
                .as_deref()
                .is_some_and(|detail| text_matches(detail, &options.capability))
    });

    if direct_match {
        score += 40;
        reasons.push(format!("direct provides match: {}", options.capability));
    } else if alias_match || evidence_match {
        score += 15;
        reasons.push("alias or evidence text match".to_owned());
    } else {
        return None;
    }

    match status {
        VerificationStatus::Verified => {
            score += 25;
            reasons.push("verified evidence".to_owned());
        }
        VerificationStatus::Unverified => {
            score -= 20;
            reasons.push("missing verification".to_owned());
        }
        VerificationStatus::Failed => unreachable!("failed candidates are excluded"),
    }

    if loaded
        .seed
        .rank
        .preferred_for
        .iter()
        .any(|capability| canonical(capability) == requested)
    {
        score += 10;
        reasons.push(format!("preferred_for {}", options.capability));
    }
    if loaded
        .seed
        .rank
        .avoid_for
        .iter()
        .any(|capability| canonical(capability) == requested)
    {
        score -= 30;
        reasons.push(format!("avoid_for {}", options.capability));
    }
    if let Some(priority) = loaded.seed.rank.default_priority {
        let normalized = i64::from(priority) / 10;
        score += normalized;
        reasons.push(format!("rank.default_priority {priority} -> +{normalized}"));
    }
    if options
        .preferred_seed
        .as_ref()
        .is_some_and(|seed_id| seed_id == &loaded.seed.id)
    {
        score += 100;
        reasons.push("explicit preferred seed".to_owned());
    }
    if options.local_only {
        if loaded.seed.risk.external_service {
            return None;
        }
        score += 20;
        reasons.push("local-only constraint satisfied".to_owned());
    } else if loaded.seed.risk.external_service || loaded.seed.risk.may_cost_money {
        score -= 10;
        reasons.push("external or paid risk requires gate".to_owned());
    }

    let risk = risk_flags(&loaded.seed);
    let required_gates = required_gates(&loaded.seed);
    Some(SearchCandidate {
        id: loaded.seed.id,
        domain: loaded.seed.domain,
        path: loaded.path.display().to_string(),
        score,
        status: "candidate",
        reasons,
        risk,
        required_gates,
    })
}

fn close_candidate_ask_policy(candidates: &[SearchCandidate]) -> Option<AskPolicy> {
    match candidates {
        [first, second, ..] if first.score - second.score <= 10 => Some(AskPolicy {
            reason: "top_candidates_within_10_points".to_owned(),
        }),
        _ => None,
    }
}

fn verify_path_lookup(command: &str) -> VerificationOutcome {
    let command_name = command.split_whitespace().next().unwrap_or(command);
    let ok = command_on_path(command_name);
    VerificationOutcome {
        check: format!("path_lookup:{command_name}"),
        ok,
        message: if ok {
            format!("{command_name} found on PATH")
        } else {
            format!("{command_name} not found on PATH")
        },
    }
}

fn verify_evidence_command(command: &str) -> VerificationOutcome {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        return VerificationOutcome {
            check: "evidence_command".to_owned(),
            ok: false,
            message: "empty evidence command".to_owned(),
        };
    }
    let output = Command::new(parts[0]).args(&parts[1..]).output();
    match output {
        Ok(output) => VerificationOutcome {
            check: format!("evidence_command:{command}"),
            ok: output.status.success(),
            message: if output.status.success() {
                "evidence command exited successfully".to_owned()
            } else {
                format!(
                    "evidence command exited with {}",
                    output.status.code().unwrap_or(-1)
                )
            },
        },
        Err(error) => VerificationOutcome {
            check: format!("evidence_command:{command}"),
            ok: false,
            message: error.to_string(),
        },
    }
}

fn command_on_path(command: &str) -> bool {
    let candidate = Path::new(command);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|path| path.join(command).is_file()))
        .unwrap_or(false)
}

fn seed_store() -> Result<PathBuf> {
    let home = match std::env::var_os("SKILLSPEC_HOME") {
        Some(home) => PathBuf::from(home),
        None => {
            let home = std::env::var_os("HOME").ok_or_else(|| Error::InvalidInput {
                message: "HOME is not set; set SKILLSPEC_HOME or HOME".to_owned(),
            })?;
            PathBuf::from(home).join(".skillspec")
        }
    };
    Ok(home.join("capabilities"))
}

fn seed_path(domain: &str, id: &str) -> Result<PathBuf> {
    Ok(seed_store()?.join(domain).join(format!("{id}.yml")))
}

fn validate_seed(seed: &CapabilitySeed) -> Result<()> {
    validate_id("capability id", &seed.id)?;
    validate_id("domain", &seed.domain)?;
    for capability in &seed.provides {
        validate_id("provides", capability)?;
    }
    if let Some(priority) = seed.rank.default_priority {
        validate_priority(Some(priority))?;
    }
    Ok(())
}

fn validate_id(field: &'static str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.starts_with('.')
        || value.contains('/')
        || value.contains('\\')
        || !value.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '-' | '.')
        })
    {
        return Err(Error::InvalidIdentifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_priority(priority: Option<u8>) -> Result<Option<u8>> {
    if priority.is_some_and(|priority| priority > 100) {
        return Err(Error::InvalidInput {
            message: "priority must be between 0 and 100".to_owned(),
        });
    }
    Ok(priority)
}

fn parse_ties(ties: &[String]) -> Result<BTreeMap<String, String>> {
    let mut parsed = BTreeMap::new();
    for tie in ties {
        let Some((key, value)) = tie.split_once('=') else {
            return Err(Error::InvalidInput {
                message: format!("tie breaker {tie:?} must use key=value"),
            });
        };
        validate_id("tie key", key)?;
        parsed.insert(key.to_owned(), value.to_owned());
    }
    Ok(parsed)
}

fn canonical(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn text_matches(text: &str, capability: &str) -> bool {
    let text = canonical(text);
    let capability = canonical(capability);
    text.contains(&capability) || capability.contains(&text)
}

fn verification_status(seed: &CapabilitySeed) -> VerificationStatus {
    seed.verification
        .as_ref()
        .map(|verification| verification.status.clone())
        .unwrap_or(VerificationStatus::Unverified)
}

fn risk_flags(seed: &CapabilitySeed) -> Vec<String> {
    let mut risk = Vec::new();
    if seed.risk.external_service {
        risk.push("external_service".to_owned());
    }
    if seed.risk.may_cost_money {
        risk.push("may_cost_money".to_owned());
    }
    risk
}

fn required_gates(seed: &CapabilitySeed) -> Vec<String> {
    let mut gates = Vec::new();
    if seed.risk.may_cost_money {
        gates.push("provider_cost_approval".to_owned());
    }
    if seed.risk.external_service {
        gates.push("external_service_approval".to_owned());
    }
    if !seed.auth.env.is_empty() {
        gates.push("secret_use_approval".to_owned());
    }
    gates
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn is_false(value: &bool) -> bool {
    !*value
}
