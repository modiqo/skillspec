use skillspec::{domain::boundary, error::Result, report};

pub(super) fn run(target: Option<String>, json: bool) -> Result<()> {
    let target = target.ok_or_else(|| skillspec::error::Error::InvalidInput {
        message: "boundary requires a target, for example `skillspec boundary ./my-skill`"
            .to_owned(),
    })?;
    let surface = boundary::analyze_target(&target)?;
    if json {
        report::json(&surface)
    } else {
        report::text(&boundary::render(&surface))
    }
}
