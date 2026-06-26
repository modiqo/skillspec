pub mod error;

pub mod spec {
    pub mod grammar;
    pub mod import_dependency_ledger;
    pub mod imports;
    pub mod model;
    pub mod parser;
}

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

pub mod lifecycle {
    pub mod durable_lifecycle;
    pub mod install;
    pub mod router;
    pub mod router_lifecycle;
    pub mod status;
    pub mod visibility;
}

pub mod features {
    pub mod capability;
    pub mod compiler;
    pub mod doctor;
    pub mod importer;
    pub mod metrics;
    pub mod run_loop;
    pub mod sensemake;
    pub mod source_map;
    pub mod workspace;
    pub mod workspace_synthesizer;
}

pub use execution::{act, align, command_path, decision, deps, progress, report, trace};
pub use features::{
    capability, compiler, doctor, importer, metrics, run_loop, sensemake, source_map, workspace,
    workspace_synthesizer,
};
pub use lifecycle::{durable_lifecycle, install, router, router_lifecycle, status, visibility};
pub use spec::{grammar, import_dependency_ledger, imports, model, parser};
