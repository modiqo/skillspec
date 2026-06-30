use crate::cli::args::{
    RouteHarnessArg, RouterPolicyAnchorArg, RouterPolicyProfileModeArg, RouterPolicyRuleModeArg,
};
use clap::Subcommand;
use skillspec::router_lifecycle;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RouterCommand {
    #[command(
        about = "Install the managed SkillSpec-backed skill-router, visibility state, index, and preparedness check"
    )]
    Install {
        /// Skill roots to scan and manage.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to write, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Reversible visibility manifest path. Defaults beside the index.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Router skill folder/name.
        #[arg(long, default_value = router_lifecycle::default_router_name())]
        router_name: String,
        /// Show changes without writing files, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Restore visibility and remove the managed skill-router",
        alias = "delete"
    )]
    Uninstall {
        /// Reversible visibility manifest path. Defaults from router config.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Router skill folder/name.
        #[arg(long, default_value = router_lifecycle::default_router_name())]
        router_name: String,
        /// SQLite index file or router directory to remove unless --keep-index is set. Defaults from router config.
        #[arg(long)]
        index: Option<PathBuf>,
        /// Preserve the index file.
        #[arg(long)]
        keep_index: bool,
        /// Show changes without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Back up and update every managed skill-router install, visibility state, and index"
    )]
    Update {
        /// Backup directory to create before mutation. Defaults under the router config directory.
        #[arg(long)]
        backup_dir: Option<PathBuf>,
        /// Show changes without writing files, backups, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Enable router mode, rebuild the index, and make routed skills explicit-only"
    )]
    Enable {
        /// Show changes without writing files, index, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Disable router mode without uninstalling it")]
    Disable {
        /// Show changes without writing files, manifest, or config.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Verify router-first readiness, repair drift, and emit native hook output")]
    Guard {
        /// Router config path. Defaults to the installed router config.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Emit harness hook JSON for UserPromptSubmit.
        #[arg(long)]
        hook: bool,
        /// Harness invoking this guard hook; emitted as route context for duplicate-root selection.
        #[arg(long, value_enum)]
        harness: Option<RouteHarnessArg>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Detect, repair, or inspect router index drift")]
    Index {
        #[command(subcommand)]
        command: RouterIndexCommand,
    },
    #[command(about = "Manage SQLite-backed router policy profiles and preference rules")]
    Policy {
        #[command(subcommand)]
        command: RouterPolicyCommand,
    },
    #[command(about = "Inspect, apply, or clear the active router policy profile")]
    Profile {
        #[command(subcommand)]
        command: RouterProfileCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RouterIndexCommand {
    #[command(about = "Repair router visibility and rebuild the index from current skill roots")]
    Refresh {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to write, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Visibility manifest whose final states should override native metadata. Defaults from router config when installed.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Detect out-of-band prose or SkillSpec-backed skill changes")]
    Status {
        /// Skill roots to scan. Repeat or pass multiple paths.
        #[arg(long = "roots", num_args = 1.., required = true)]
        roots: Vec<PathBuf>,
        /// SQLite index file to inspect, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Visibility manifest whose final states should override native metadata.
        #[arg(long)]
        visibility_manifest: Option<PathBuf>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RouterPolicyCommand {
    #[command(about = "Create router policy tables in an existing skill index")]
    Init {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "List router policy profiles stored in the index")]
    List {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Show one profile's router policy rules, or all rules when no profile is active"
    )]
    Show {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Profile to show. Defaults to the active profile.
        #[arg(long)]
        profile: Option<String>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Get one router policy profile or rule by id")]
    Get {
        /// Profile name or rule id.
        id: String,
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Create or update a router policy profile")]
    SetProfile {
        /// Profile name.
        name: String,
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Profile behavior.
        #[arg(long, value_enum, default_value = "route")]
        mode: RouterPolicyProfileModeArg,
        /// Make this profile active.
        #[arg(long)]
        active: bool,
        /// Treat policy warnings as strict authoring intent.
        #[arg(long)]
        strict: bool,
        /// Human description for audit/review.
        #[arg(long)]
        description: Option<String>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Create or replace a router policy preference rule")]
    SetRule {
        /// Rule id.
        id: String,
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Profile that owns this rule.
        #[arg(long)]
        profile: String,
        /// Higher priority rules apply first.
        #[arg(long, default_value_t = 0)]
        priority: i64,
        /// Soft rules adjust score; hard rules strongly promote matching allow/prefer targets.
        #[arg(long, value_enum, default_value = "soft")]
        mode: RouterPolicyRuleModeArg,
        /// Whether this rule can satisfy the activation-anchor gate.
        #[arg(long, value_enum, default_value = "none")]
        anchor: RouterPolicyAnchorArg,
        /// Disable the rule while leaving it stored.
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        enabled: bool,
        /// Match when any phrase appears in the query. Repeat for multiple phrases.
        #[arg(long = "when-any")]
        when_any: Vec<String>,
        /// Match only when every phrase appears in the query. Repeat for multiple phrases.
        #[arg(long = "when-all")]
        when_all: Vec<String>,
        /// Do not match when any phrase appears in the query. Repeat for multiple phrases.
        #[arg(long = "when-none")]
        when_none: Vec<String>,
        /// Prefer a target, such as skill:name, tag:name, source:root, or has_skill_spec:true.
        #[arg(long = "prefer")]
        prefer: Vec<String>,
        /// Allow a target in passthrough mode.
        #[arg(long = "allow")]
        allow: Vec<String>,
        /// Suppress a target.
        #[arg(long = "suppress")]
        suppress: Vec<String>,
        /// Forbid a target.
        #[arg(long = "forbid")]
        forbid: Vec<String>,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Remove one router policy rule")]
    RemoveRule {
        /// Rule id.
        id: String,
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Explain how a profile changes route selection for one query")]
    Explain {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// User task text to route.
        #[arg(long, allow_hyphen_values = true)]
        query: String,
        /// Profile to apply instead of the active profile.
        #[arg(long)]
        profile: Option<String>,
        /// Number of candidates to return.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RouterProfileCommand {
    #[command(about = "Show the active router policy profile")]
    Status {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Make a router policy profile active")]
    Apply {
        /// Profile name.
        profile: String,
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Show changes without writing active profile state.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Clear the active router policy profile")]
    Clear {
        /// SQLite index file, or a router directory containing skill-index.sqlite.
        #[arg(long)]
        index: PathBuf,
        /// Show changes without writing active profile state.
        #[arg(long)]
        dry_run: bool,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}
