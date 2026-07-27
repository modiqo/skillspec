use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum BoundaryCommand {
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
}
