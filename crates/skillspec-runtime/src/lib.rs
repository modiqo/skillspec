//! Internal runtime execution modules for the SkillSpec CLI.
//!
//! This crate is an implementation boundary used by the workspace. It is not a
//! stable Rust API.

pub mod act;
pub mod align;
pub mod command_path;
pub mod decision;
pub mod deps;
pub mod guide;
pub mod progress;
pub mod report;
pub mod run_loop;
pub mod trace;

pub use skillspec_core::error;
