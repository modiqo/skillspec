//! Effect-surface enumeration and least-privilege boundary proposals.
//!
//! This crate answers one question about a skill package:
//!
//! ```text
//! If an agent executed this skill, what could it reach, and what is the
//! smallest permission set that still lets the skill work?
//! ```
//!
//! It enumerates rather than classifies. Where a scanner asks whether a given
//! call is malicious - a question that cannot be answered from source text - an
//! enumerator asks only what the package touches, including the ordinary parts.
//! The consequence is the failure mode: a classifier that misses something fails
//! open, whereas an enumerator that misses something fails closed *provided the
//! emitted policy is enforced by a substrate with deny-by-default semantics*.
//! That proviso is load-bearing and is stated wherever the property is claimed.
//!
//! SkillSpec does not enforce anything by itself. A boundary proposal is a
//! policy artifact for a harness permission system, a guard hook, or a network
//! policy to apply.
//!
//! Reports never claim intent. They state what a package can reach and where the
//! evidence is.
//!
//! Design: `docs/design/security/`.
//!
//! This crate is an implementation boundary used by the workspace. It is not a
//! stable Rust API.

pub mod bounds;
pub mod effect;
pub mod normalize;
pub mod sanitize;

pub use bounds::{Bounds, Budget};
pub use effect::{
    Confidence, EffectClass, EffectEvidence, EffectObservation, EffectOrigin, EffectTarget,
    PathClass, Reach, TargetResolution,
};
pub use sanitize::Preview;
