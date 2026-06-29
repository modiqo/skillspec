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
    pub use skillspec_runtime::{
        act, align, command_path, decision, deps, progress, report, trace,
    };
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
    pub mod run_loop;
    pub mod sensemake;
    pub mod workspace;

    pub use skillspec_authoring::{
        compiler, git_context, importer, metrics, port_one_shot, workspace_synthesizer,
    };
    pub use skillspec_doctor as doctor;
    pub use skillspec_doctor::{remote_source, source_map};
}

#[doc(hidden)]
pub use features::{capability, git_context, run_loop, sensemake, workspace};
#[doc(hidden)]
pub use lifecycle::{durable_lifecycle, install, router, router_lifecycle, status, visibility};
#[doc(hidden)]
pub use skillspec_authoring::{compiler, importer, metrics, port_one_shot, workspace_synthesizer};
#[doc(hidden)]
pub use skillspec_core::{grammar, import_dependency_ledger, imports, model, parser};
#[doc(hidden)]
pub use skillspec_doctor as doctor;
#[doc(hidden)]
pub use skillspec_doctor::{remote_source, source_map};
#[doc(hidden)]
pub use skillspec_runtime::{
    act, align, command_path, decision, deps, guide, progress, report, trace,
};
