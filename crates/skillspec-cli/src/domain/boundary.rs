use crate::{boundary, error};

pub fn analyze_target(target: &str) -> error::Result<boundary::EffectSurface> {
    boundary::analyze_target(target)
}

pub fn render(surface: &boundary::EffectSurface) -> String {
    boundary::render(surface)
}
