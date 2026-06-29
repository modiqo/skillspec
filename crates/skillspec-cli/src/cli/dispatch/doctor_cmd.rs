use skillspec::{doctor, error::Result, report};

pub(super) fn run(path: String, json: bool, html: bool, markdown: bool) -> Result<()> {
    let doctor_report = doctor::inspect_target(&path)?;
    if json {
        report::json(&doctor_report)
    } else if html {
        report::text(&doctor::render_html(&doctor_report))
    } else if markdown {
        report::text(&doctor::render_markdown(&doctor_report))
    } else {
        report::text(&doctor::render(&doctor_report))
    }
}
