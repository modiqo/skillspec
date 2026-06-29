//! Implementation crate for the `skillspec` command-line tool.
//!
//! SkillSpec is primarily distributed as a CLI. The supported integration
//! surface is the `skillspec` binary and the documented command behavior in the
//! project repository.
//!
//! The Rust modules in this crate are public so the binary and integration
//! tests can share implementation code, but they are not yet a stable library
//! API. They are hidden from generated API documentation until SkillSpec has an
//! intentionally designed Rust API with compatibility guarantees.

#[doc(hidden)]
pub use skillspec_core::error;

#[doc(hidden)]
pub mod domain;

#[doc(hidden)]
pub mod spec {
    pub use skillspec_core::spec::{grammar, import_dependency_ledger, imports, model, parser};
}

#[doc(hidden)]
pub mod execution {
    pub mod act;
    pub mod align;
    pub mod command_path;
    pub mod decision;
    pub mod deps;
    pub mod progress;
    pub mod report;
    pub mod trace;
}

#[doc(hidden)]
pub mod lifecycle {
    pub mod durable_lifecycle;
    pub mod install;
    pub mod router;
    pub mod router_lifecycle;
    pub mod status;
    pub mod visibility;
}

#[doc(hidden)]
pub mod features {
    pub mod capability;
    pub mod compiler;
    pub mod doctor;
    pub mod git_context;
    pub mod guide;
    pub mod importer;
    pub mod metrics;
    pub mod port_one_shot;
    pub mod remote_source;
    pub mod run_loop;
    pub mod sensemake;
    pub mod source_map;
    pub mod workspace;
    pub mod workspace_synthesizer;
}

#[doc(hidden)]
pub use execution::{act, align, command_path, decision, deps, progress, report, trace};
#[doc(hidden)]
pub use features::{
    capability, compiler, doctor, git_context, guide, importer, metrics, port_one_shot,
    remote_source, run_loop, sensemake, source_map, workspace, workspace_synthesizer,
};
#[doc(hidden)]
pub use lifecycle::{durable_lifecycle, install, router, router_lifecycle, status, visibility};
#[doc(hidden)]
pub use skillspec_core::{grammar, import_dependency_ledger, imports, model, parser};
