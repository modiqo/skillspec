//! Internal core contract modules for the SkillSpec CLI.
//!
//! This crate is an implementation boundary used by the workspace. It is not a
//! stable Rust API.

pub mod error;

pub mod spec {
    pub mod grammar;
    pub mod import_dependency_ledger;
    pub mod imports;
    pub mod model;
    pub mod parser;
}

pub use spec::{grammar, import_dependency_ledger, imports, model, parser};
