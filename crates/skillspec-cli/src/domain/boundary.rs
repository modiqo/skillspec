use crate::{boundary, error};

pub use boundary::{EffectSurface, EmitTarget, Proposal};

pub fn analyze_target(target: &str) -> error::Result<EffectSurface> {
    boundary::analyze_target(target)
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
