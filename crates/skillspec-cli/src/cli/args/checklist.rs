use super::ChecklistStageArg;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum DoctorCommand {
    #[command(
        about = "Generate a shape-specific doctor checklist",
        long_about = "Inspect a local or public GitHub skill source and generate a concrete shape-specific checklist for single-skill, multi-skill, plugin-shaped, or non-skill sources. This is a read-only planning surface: it does not import, compile, install, or write proof ledgers."
    )]
    Checklist {
        /// Local source folder/file or public GitHub skill/repo URI.
        source: String,
        /// Checklist stage to render.
        #[arg(long, value_enum, default_value_t = ChecklistStageArg::Entry)]
        stage: ChecklistStageArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum ImportCommand {
    #[command(
        about = "Generate an import checklist for a source or workspace",
        long_about = "Generate a concrete import checklist for a source root or skillspec.workspace.yml manifest. The checklist names the detected shape, activation policy, package/file/block repeat loops, exact SkillSpec commands, forbids, and evidence paths. It is a planning/checklist surface and does not promote package semantics by itself."
    )]
    Checklist {
        /// Source root or skillspec.workspace.yml.
        target: String,
        /// Workspace build root when checking workspace package loop or exit stages.
        #[arg(long = "build-root")]
        build_root: Option<PathBuf>,
        /// Checklist stage to render.
        #[arg(long, value_enum, default_value_t = ChecklistStageArg::Entry)]
        stage: ChecklistStageArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RunCommand {
    #[command(
        about = "Generate a run checklist for a spec or guided run",
        long_about = "Generate a concrete execution checklist from a skill.spec.yml or an existing run directory. For run directories, it reads guide-state.json and exposes the selected route, current phase, open requirements, forbids, allowed commands, repeat-until condition, and completion proof cues."
    )]
    Checklist {
        /// Path to skill.spec.yml or an existing guided run directory.
        target: PathBuf,
        /// Checklist stage to render.
        #[arg(long, value_enum, default_value_t = ChecklistStageArg::Loop)]
        stage: ChecklistStageArg,
        /// Emit JSON instead of a concise human report.
        #[arg(long)]
        json: bool,
    },
}
