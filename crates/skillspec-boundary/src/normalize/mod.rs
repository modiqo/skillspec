//! Turning source text into comparable effect targets.
//!
//! Normalizers are the part of this crate most likely to be subtly wrong, and
//! the cheapest to test exhaustively, so each one carries a table-driven test
//! suite alongside it.
//!
//! They share one rule: when source text does not determine a target, say so
//! with [`crate::effect::TargetResolution`] rather than guessing. A guessed
//! target becomes a grant, and a wrong grant is worse than an honest gap.

pub mod argv;
pub mod env;
pub mod path;
pub mod url;
