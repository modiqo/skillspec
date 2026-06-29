use crate::cli::args::SourceCommand;
use skillspec::{error::Result, remote_source, report, source_map};

pub(super) fn run(command: SourceCommand) -> Result<()> {
    match command {
        SourceCommand::Stage {
            uri,
            out,
            no_detect_candidates,
            json,
        } => {
            let stage_report =
                remote_source::stage_remote_source(&uri, out.as_deref(), !no_detect_candidates)?;
            if json {
                report::json(&stage_report)?;
            } else {
                report::text(&remote_source::render_stage_report(&stage_report))?;
            }
        }
        SourceCommand::Map { path, out, json } => {
            let report = source_map::create_source_map(&path, &out)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&source_map::render_write_report(&report))?;
            }
        }
        SourceCommand::Query {
            map,
            handle,
            view,
            json,
        } => {
            let report = source_map::query(&map, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&source_map::render_query(&report))?;
            }
        }
        SourceCommand::Coverage { map, json } => {
            let map = source_map::load(&map)?;
            if json {
                report::json(&map.coverage)?;
            } else {
                report::text(&source_map::render_coverage(&map.coverage))?;
            }
        }
        SourceCommand::Stale { map, root, json } => {
            let report = source_map::stale(&map, root.as_deref())?;
            let ok = report.ok;
            if json {
                report::json(&report)?;
            } else {
                report::text(&source_map::render_stale(&report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
