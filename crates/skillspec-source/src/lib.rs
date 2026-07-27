//! Internal source-package reading modules for the SkillSpec CLI.
//!
//! This crate owns the machinery for turning a skill source package on disk,
//! or a public remote target, into structured records that analysis crates can
//! consume: the source map, and the primitives for staging a remote checkout.
//!
//! It deliberately contains no analysis, no scoring, and no reporting. Those
//! belong to the crates that read from here, which is what keeps two analysis
//! products from depending on each other for plumbing.
//!
//! This crate is an implementation boundary used by the workspace. It is not a
//! stable Rust API.

pub mod remote;
pub mod source_map;
