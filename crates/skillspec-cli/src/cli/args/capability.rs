use clap::Subcommand;

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(in crate::cli) enum CapabilityCommand {
    #[command(about = "Show the local capability seed store path")]
    Store,
    #[command(about = "Create or update a local capability seed")]
    Add {
        /// Stable seed id, such as preferred-voice-cli.
        id: String,
        /// Capability domain folder, such as voice or pdf.
        #[arg(long)]
        domain: String,
        /// Seed kind, such as cli, adapter, script, or flow.
        #[arg(long)]
        kind: String,
        /// CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Local script path.
        #[arg(long)]
        script: Option<String>,
        /// Capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        provides: Vec<String>,
        /// User phrase alias for this seed. Repeat for multiple aliases.
        #[arg(long)]
        alias: Vec<String>,
        /// Default priority from 0 to 100, used only as a tie-breaker.
        #[arg(long)]
        priority: Option<u8>,
        /// Capability this seed is preferred for. Repeat for multiple capabilities.
        #[arg(long)]
        preferred_for: Vec<String>,
        /// Capability this seed should avoid. Repeat for multiple capabilities.
        #[arg(long)]
        avoid_for: Vec<String>,
        /// Tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long = "tie")]
        ties: Vec<String>,
        /// Environment variable used for auth. Repeat for multiple vars.
        #[arg(long)]
        auth_env: Vec<String>,
        /// Mark this seed as using an external service.
        #[arg(long)]
        external_service: bool,
        /// Mark this seed as potentially spending provider credits or money.
        #[arg(long)]
        may_cost_money: bool,
        /// Evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        evidence_command: Vec<String>,
        /// Suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
    },
    #[command(
        about = "Patch an existing local capability seed without rewriting unspecified fields"
    )]
    Update {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Replace seed kind.
        #[arg(long)]
        kind: Option<String>,
        /// Set CLI command name or path.
        #[arg(long)]
        command: Option<String>,
        /// Clear CLI command.
        #[arg(long)]
        clear_command: bool,
        /// Set adapter id or name.
        #[arg(long)]
        adapter: Option<String>,
        /// Clear adapter id or name.
        #[arg(long)]
        clear_adapter: bool,
        /// Set local script path.
        #[arg(long)]
        script: Option<String>,
        /// Clear local script path.
        #[arg(long)]
        clear_script: bool,
        /// Add a capability provided by this seed. Repeat for multiple capabilities.
        #[arg(long)]
        add_provides: Vec<String>,
        /// Remove a provided capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_provides: Vec<String>,
        /// Add a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        add_alias: Vec<String>,
        /// Remove a user phrase alias. Repeat for multiple aliases.
        #[arg(long)]
        remove_alias: Vec<String>,
        /// Set default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
        /// Clear default priority.
        #[arg(long)]
        clear_priority: bool,
        /// Add a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        add_preferred_for: Vec<String>,
        /// Remove a preferred capability. Repeat for multiple capabilities.
        #[arg(long)]
        remove_preferred_for: Vec<String>,
        /// Add an avoided capability. Useful when a seed stops working for a task.
        #[arg(long)]
        add_avoid_for: Vec<String>,
        /// Remove an avoided capability.
        #[arg(long)]
        remove_avoid_for: Vec<String>,
        /// Add or replace tie-breaker metadata as key=value. Repeat for multiple entries.
        #[arg(long)]
        add_tie: Vec<String>,
        /// Remove tie-breaker metadata by key. Repeat for multiple entries.
        #[arg(long)]
        remove_tie: Vec<String>,
        /// Add an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        add_auth_env: Vec<String>,
        /// Remove an auth environment variable. Repeat for multiple vars.
        #[arg(long)]
        remove_auth_env: Vec<String>,
        /// Set external service risk flag.
        #[arg(long)]
        external_service: Option<bool>,
        /// Set provider cost risk flag.
        #[arg(long)]
        may_cost_money: Option<bool>,
        /// Add evidence command, such as "tool --help". Repeat for multiple checks.
        #[arg(long)]
        add_evidence_command: Vec<String>,
        /// Remove an evidence command. Repeat for multiple checks.
        #[arg(long)]
        remove_evidence_command: Vec<String>,
        /// Set suggested domain SkillSpec id to generate after a successful trace.
        #[arg(long)]
        suggested_skill_id: Option<String>,
        /// Clear suggested domain SkillSpec id.
        #[arg(long)]
        clear_suggested_skill_id: bool,
        /// Mark verification status unverified without running checks.
        #[arg(long, conflicts_with = "mark_failed")]
        mark_unverified: bool,
        /// Mark verification status failed without running checks.
        #[arg(long, conflicts_with = "mark_unverified")]
        mark_failed: bool,
    },
    #[command(about = "List local capability seeds")]
    List {
        /// Limit results to one domain.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Search and rank local capability seeds for one capability/domain pair")]
    Search {
        /// Capability to search for, such as text_to_speech.
        capability: String,
        /// Limit results to one domain. If no candidates are found, callers should search related domains before using an unseeded fallback.
        #[arg(long)]
        domain: Option<String>,
        /// Include ranking reasons in the JSON output.
        #[arg(long)]
        explain: bool,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
        /// Exclude external service candidates.
        #[arg(long)]
        local_only: bool,
        /// Explicitly preferred seed id for this search.
        #[arg(long)]
        preferred_seed: Option<String>,
    },
    #[command(about = "Inspect one local capability seed")]
    Inspect {
        /// Seed id to inspect.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Verify one local capability seed's evidence")]
    Verify {
        /// Seed id to verify.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Emit JSON output. Accepted for command symmetry; JSON is always emitted.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Update preferred capability and priority metadata for a seed")]
    Prefer {
        /// Seed id to update.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
        /// Capability this seed should be preferred for.
        #[arg(long = "for")]
        for_capability: String,
        /// Default priority from 0 to 100.
        #[arg(long)]
        priority: Option<u8>,
    },
    #[command(about = "Remove one local capability seed")]
    Remove {
        /// Seed id to remove.
        id: String,
        /// Disambiguating domain when the id appears in multiple domains.
        #[arg(long)]
        domain: Option<String>,
    },
    #[command(about = "Scan for seed proposals")]
    Scan,
}
