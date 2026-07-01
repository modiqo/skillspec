use crate::{doctor, error};

pub fn inspect_target(target: &str) -> error::Result<doctor::DoctorReport> {
    doctor::inspect_target(target)
}

pub fn render(report: &doctor::DoctorReport) -> String {
    doctor::render(report)
}

pub fn render_html(report: &doctor::DoctorReport) -> String {
    doctor::render_html(report)
}

pub fn render_markdown(report: &doctor::DoctorReport) -> String {
    doctor::render_markdown(report)
}
