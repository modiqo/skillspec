//! Resource limits for analyzing untrusted packages.
//!
//! The hosted path runs this analysis against arbitrary public repositories, so
//! extraction is bounded. Bounds are *reported*, never applied silently: a
//! truncated analysis did not determine the whole surface, and a report that hid
//! that would understate the effect surface in exactly the direction this design
//! is trying to avoid.
//!
//! See `docs/design/security/36-skill-effect-surface.md`.

use serde::Serialize;

/// Limits applied while reading a package.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Bounds {
    pub max_files: usize,
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_line_bytes: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            max_files: 2_000,
            max_file_bytes: 2 * 1024 * 1024,
            max_total_bytes: 64 * 1024 * 1024,
            max_line_bytes: 64 * 1024,
        }
    }
}

/// Why a file was not analyzed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// File is larger than [`Bounds::max_file_bytes`].
    FileTooLarge,
    /// The file budget was already exhausted.
    FileBudgetExhausted,
    /// The byte budget was already exhausted.
    ByteBudgetExhausted,
    /// Contents are not text.
    Binary,
}

impl SkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileTooLarge => "file_too_large",
            Self::FileBudgetExhausted => "file_budget_exhausted",
            Self::ByteBudgetExhausted => "byte_budget_exhausted",
            Self::Binary => "binary",
        }
    }
}

/// One file the analysis did not read.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: SkipReason,
}

/// Tracks consumption against [`Bounds`] and records what was skipped.
#[derive(Clone, Debug)]
pub struct Budget {
    bounds: Bounds,
    files_read: usize,
    bytes_read: usize,
    skipped: Vec<SkippedFile>,
}

impl Budget {
    pub fn new(bounds: Bounds) -> Self {
        Self {
            bounds,
            files_read: 0,
            bytes_read: 0,
            skipped: Vec::new(),
        }
    }

    /// Ask to read a file of `bytes` length.
    ///
    /// Returns whether it may be read, recording a skip entry when it may not.
    pub fn admit(&mut self, path: &str, bytes: usize) -> bool {
        let reason = if self.files_read >= self.bounds.max_files {
            Some(SkipReason::FileBudgetExhausted)
        } else if bytes > self.bounds.max_file_bytes {
            Some(SkipReason::FileTooLarge)
        } else if self.bytes_read.saturating_add(bytes) > self.bounds.max_total_bytes {
            Some(SkipReason::ByteBudgetExhausted)
        } else {
            None
        };

        match reason {
            Some(reason) => {
                self.skip(path, reason);
                false
            }
            None => {
                self.files_read += 1;
                self.bytes_read += bytes;
                true
            }
        }
    }

    /// Record a file skipped for a reason the budget did not decide.
    pub fn skip(&mut self, path: &str, reason: SkipReason) {
        self.skipped.push(SkippedFile {
            path: path.to_owned(),
            reason,
        });
    }

    /// Truncate an over-long line to the configured limit.
    pub fn clamp_line<'a>(&self, line: &'a str) -> &'a str {
        if line.len() <= self.bounds.max_line_bytes {
            return line;
        }
        let mut end = self.bounds.max_line_bytes;
        while end > 0 && !line.is_char_boundary(end) {
            end -= 1;
        }
        &line[..end]
    }

    /// Whether a resource limit truncated the analysis, so it did not read
    /// everything it should have.
    ///
    /// A skipped *binary* file does not count: an image or archive holds no
    /// effects, and its absence hides nothing, so a skill that ships assets is
    /// not incomplete for that reason. Only a file-count, file-size, or byte
    /// budget that fired means the surface was genuinely cut short.
    pub fn truncated(&self) -> bool {
        self.skipped
            .iter()
            .any(|file| file.reason != SkipReason::Binary)
    }

    pub fn files_read(&self) -> usize {
        self.files_read
    }

    pub fn skipped(&self) -> &[SkippedFile] {
        &self.skipped
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(Bounds::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{Bounds, Budget, SkipReason};

    fn small_bounds() -> Bounds {
        Bounds {
            max_files: 2,
            max_file_bytes: 100,
            max_total_bytes: 150,
            max_line_bytes: 8,
        }
    }

    #[test]
    fn files_within_every_bound_are_admitted() {
        let mut budget = Budget::new(small_bounds());
        assert!(budget.admit("a.sh", 50));
        assert_eq!(budget.files_read(), 1);
        assert!(!budget.truncated());
    }

    #[test]
    fn an_oversized_file_is_skipped_and_recorded() {
        let mut budget = Budget::new(small_bounds());
        assert!(!budget.admit("big.bin", 500));
        assert_eq!(budget.skipped()[0].reason, SkipReason::FileTooLarge);
        assert!(budget.truncated());
    }

    #[test]
    fn a_skipped_binary_does_not_make_the_analysis_incomplete() {
        // A skill that ships images is not "incomplete" for skipping them; a
        // PNG holds no effects.
        let mut budget = Budget::new(small_bounds());
        budget.skip("logo.png", SkipReason::Binary);
        assert!(!budget.truncated());
        assert_eq!(budget.skipped().len(), 1);
    }

    #[test]
    fn the_file_count_budget_is_enforced() {
        let mut budget = Budget::new(small_bounds());
        assert!(budget.admit("a", 10));
        assert!(budget.admit("b", 10));
        assert!(!budget.admit("c", 10));
        assert_eq!(budget.skipped()[0].reason, SkipReason::FileBudgetExhausted);
    }

    #[test]
    fn the_total_byte_budget_is_enforced() {
        let mut budget = Budget::new(small_bounds());
        assert!(budget.admit("a", 100));
        assert!(!budget.admit("b", 100));
        assert_eq!(budget.skipped()[0].reason, SkipReason::ByteBudgetExhausted);
    }

    #[test]
    fn a_skipped_file_does_not_consume_budget() {
        let mut budget = Budget::new(small_bounds());
        assert!(!budget.admit("big", 500));
        assert!(budget.admit("small", 50));
        assert_eq!(budget.files_read(), 1);
    }

    #[test]
    fn long_lines_clamp_on_a_character_boundary() {
        let budget = Budget::new(small_bounds());
        // Multi-byte characters must not be split mid-codepoint.
        let clamped = budget.clamp_line("ααααααααα");
        assert!(clamped.len() <= 8);
        assert!("ααααααααα".starts_with(clamped));
    }

    #[test]
    fn short_lines_pass_through_untouched() {
        let budget = Budget::new(small_bounds());
        assert_eq!(budget.clamp_line("git st"), "git st");
    }

    #[test]
    fn default_bounds_are_generous_enough_for_real_packages() {
        let mut budget = Budget::default();
        assert!(budget.admit("SKILL.md", 40 * 1024));
        assert!(!budget.truncated());
    }
}
