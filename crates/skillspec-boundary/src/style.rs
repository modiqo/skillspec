//! Minimal ANSI styling, emitted only when the output can render it.
//!
//! Color is an aid, never load-bearing: the same report reads correctly with it
//! stripped. So styling is applied only when stdout is a terminal and the
//! caller has not set `NO_COLOR`, and every styled token degrades to its plain
//! text otherwise.

use std::io::IsTerminal;

/// A small palette, named by intent rather than by color.
///
/// The severity colors follow the convention security tooling uses so the
/// meaning carries at a glance: critical and high in red, medium in orange, low
/// in blue.
#[derive(Clone, Copy)]
pub enum Style {
    /// A critical finding — bold red.
    Alarm,
    /// A high finding — red.
    Danger,
    /// A medium finding — orange.
    Warn,
    /// A low finding — blue.
    Info,
    /// Something benign or reassuring — green.
    Ok,
    /// A structural marker — cyan.
    Accent,
    /// A section heading — bold.
    Heading,
    /// Secondary text — dim.
    Muted,
}

impl Style {
    fn code(self) -> &'static str {
        match self {
            Style::Alarm => "1;31",
            Style::Danger => "31",
            Style::Warn => "38;5;208",
            Style::Info => "34",
            Style::Ok => "32",
            Style::Accent => "36",
            Style::Heading => "1",
            Style::Muted => "2",
        }
    }
}

/// Whether ANSI color should be emitted: stdout is a terminal and `NO_COLOR` is
/// unset. The [`NO_COLOR` convention](https://no-color.org/) is honored.
pub fn colors_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Wrap `text` in an ANSI style when `enabled`, else return it unchanged.
pub fn paint(text: &str, style: Style, enabled: bool) -> String {
    if enabled {
        format!("\u{1b}[{}m{text}\u{1b}[0m", style.code())
    } else {
        text.to_owned()
    }
}
