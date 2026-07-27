use crate::{boundary, error};

pub use boundary::map::{self, SurfaceMap};
pub use boundary::workspace::{self, WorkspaceSurface};
pub use boundary::{
    Analysis, CheckMode, CheckOutcome, DriftReport, EffectSurface, EmitTarget, Proposal,
};

pub fn reveal_payloads(target: &str, out: &str) -> error::Result<()> {
    boundary::reveal_payloads(target, out)
}

pub fn analyze_target(target: &str) -> error::Result<EffectSurface> {
    boundary::analyze_target(target)
}

pub fn analyze_any(target: &str) -> error::Result<Analysis> {
    boundary::analyze_any(target)
}

pub fn surface_map(target: &str) -> error::Result<SurfaceMap> {
    boundary::surface_map(target)
}

pub fn render_surface_map(m: &SurfaceMap) -> String {
    boundary::map::render(m)
}

pub fn render(surface: &EffectSurface) -> String {
    boundary::render(surface)
}

pub fn compile(surface: &EffectSurface) -> Proposal {
    boundary::compile(surface)
}

pub fn parse_format(value: &str) -> error::Result<EmitTarget> {
    EmitTarget::parse(value)
}

pub fn emit(proposal: &Proposal, target: EmitTarget) -> error::Result<String> {
    boundary::emit(proposal, target)
}

pub fn diff_against(target: &str, git_ref: &str) -> error::Result<DriftReport> {
    boundary::diff_against(target, git_ref)
}

pub fn render_drift(report: &DriftReport) -> String {
    boundary::drift::render(report)
}

pub fn check(
    target: &str,
    mode: CheckMode,
    fail_on_incomplete: bool,
) -> error::Result<(CheckOutcome, String)> {
    boundary::check(target, mode, fail_on_incomplete)
}
