use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum BoundaryCommand {
    #[command(
        about = "Map the shape of a skill folder before analyzing it (the Structure view)",
        long_about = "Build the surface map of a folder of skills without analyzing effects: which skills are present, which files each one references (resources), which files nothing references (orphans - where a payload or directive can hide), and which skills reference other skills. Skills that reference each other form a connected group; the rest are independent. This is the Structure half of what `gate` shows; run it first to understand a repository's shape before the security analysis."
    )]
    Map {
        /// Local folder of skills, or public git skill URL.
        path: String,
        /// Emit machine-readable JSON instead of the formatted map.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Analyze what a skill could reach, ranked by risk (the Security Analysis view)",
        long_about = "Run the security analysis on its own: for each skill, what it could reach if executed, ranked by severity (critical/high/medium/low) with scope-awareness (a skill confined to its own directory is low risk; reaching outside it - a home or absolute path, an `..` escape, the network, another skill's files - is what earns review). Each finding states how bad it is, why, what happens if it runs, and a link to the exact line. This is the Security Analysis half of what `gate` shows, without the structure map or the install prompt. `--json` emits the structured findings and per-severity tally for tooling."
    )]
    Assess {
        /// Local skill folder, or public git skill URL.
        path: String,
        /// Emit machine-readable JSON instead of the formatted analysis.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Emit a least-privilege boundary proposal in a policy format",
        long_about = "Compile the enumerated effect surface into a deny-by-default permission set and render it into a policy grammar. Supported formats: skillspec (a tool_boundary block for skill.spec.yml), egress-allowlist (a host list for a proxy or container network policy), and json (the proposal itself). Every supported format can express deny-by-default; a format that cannot would not give the fail-closed property the analysis depends on. Effects whose target could not be determined never become grants, and a proposal built from an incomplete surface says so in the emitted artifact. SkillSpec does not enforce the emitted policy."
    )]
    Emit {
        /// Local skill folder, public GitHub skill folder URL, or public GitHub repo URI.
        path: String,
        /// Policy grammar to render: skillspec, egress-allowlist, or json.
        #[arg(long, default_value = "skillspec")]
        format: String,
        /// Write the artifact to a file instead of stdout.
        #[arg(long, short = 'o')]
        out: Option<String>,
    },
    #[command(
        about = "Compare a skill's effect surface against a prior git revision",
        long_about = "Analyze a local skill and the same package at a prior git revision, and report what changed in its effect surface. The target must be inside a git working tree; the prior revision is materialized in a detached worktree and removed afterwards, disturbing neither the working tree nor the index. A new network host, a new sensitive-path read, a capability that moved into an unreferenced file, or a target that became undeterminable are reported as changes that warrant review. Two revisions analyzed by different extractor versions are reported as non-comparable rather than as spurious growth."
    )]
    Diff {
        /// Local skill folder inside a git working tree.
        path: String,
        /// Git revision to compare against, such as HEAD~1 or a tag.
        #[arg(long)]
        against: String,
        /// Emit machine-readable JSON instead of the formatted report.
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Exit non-zero when a skill's effect surface meets a gate, for CI",
        long_about = "Evaluate a skill for a CI gate. With no --against, performs an absolute review of the whole surface: exit 1 when it reads a sensitive path or reaches the network, exit 0 otherwise. With --against <ref>, gates on drift since that revision instead, which is the right rule for an update - a skill hostile from its first commit shows no drift, so absolute review is used for first contact and drift for updates. Exit 2 is reserved for an incomplete surface (a target that could not be fully determined) or a non-comparable revision pair, which is a distinct condition from a determined-and-concerning surface. Exit 3 on analysis error."
    )]
    Check {
        /// Local skill folder; must be inside a git working tree when --against is used.
        path: String,
        /// Gate on drift since this git revision instead of an absolute review.
        #[arg(long)]
        against: Option<String>,
        /// Also fail (exit 2) when the surface is incomplete.
        #[arg(long)]
        fail_on_incomplete: bool,
    },
    #[command(
        about = "Assess a skill or plugin before installing it, then run the install on approval",
        long_about = "A pre-install gate. A skill or plugin ships in a git repository and is copied out of it on install, so the repository is assessed before anything lands on disk. The gate maps the target (the tree of skills, resources, and orphan files), reports what any skill could reach if executed, and - when there are findings - asks you to confirm before proceeding. Pass the real install command with --then and the gate runs it only on approval; without --then it just reports and approves. --yes proceeds without a prompt (for scripts). With findings and no terminal to confirm, the gate refuses. This is the human-driven counterpart to `guard`, which intercepts an agent-driven install at the PreToolUse hook."
    )]
    Gate {
        /// Skill/plugin repo to assess: a local folder or a public git URL/owner-repo.
        path: String,
        /// Install command to run on approval, e.g. "claude plugin install rote-onboard@rote-skills".
        #[arg(long)]
        then: Option<String>,
        /// Proceed without an interactive prompt.
        #[arg(long)]
        yes: bool,
    },
    #[command(
        about = "Manage the boundary guard hook that enforces reviewed policies",
        long_about = "Install and operate a managed PreToolUse hook that applies reviewed boundary policies to skills you already have. The guard needs no skill.spec.yml, no compile, and no change to the guarded skill. It installs in observe mode by default - recording every intercepted tool call and whether an approved grant covers it, blocking nothing - because a control that blocks against an unvalidated policy is uninstalled within the hour. Promote to enforce once the decision log shows the policy covering real calls. A PreToolUse call cannot be attributed to a single skill, so enforcement is against the union of every approved policy's grants. SkillSpec does not sandbox: the guard is an advisory gate at a lifecycle event the harness exposes, and a guard that cannot run is reported in status rather than silently permitting."
    )]
    Guard {
        #[command(subcommand)]
        command: GuardCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
pub(in crate::cli) enum GuardCommand {
    #[command(about = "Install the managed PreToolUse hook (observe mode)")]
    Install,
    #[command(about = "Approve a skill's boundary proposal as a stored policy")]
    Add {
        /// Local skill folder, or public GitHub skill URL.
        path: String,
    },
    #[command(about = "Show guard config, mode, and stored policies")]
    Status,
    #[command(about = "Set the guard mode: observe, prompt, or enforce")]
    Mode {
        /// observe, prompt, or enforce.
        mode: String,
    },
    #[command(about = "Read the local decision log")]
    Log {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Remove the managed hook and guard config")]
    Uninstall,
    #[command(
        about = "PreToolUse entrypoint: reads a payload on stdin, prints a decision",
        hide = true
    )]
    Hook,
}
