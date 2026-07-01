use crate::cli::args::SourceCommand;
use skillspec::{domain::authoring, error::Result, report};

pub(super) fn run(command: SourceCommand) -> Result<()> {
    match command {
        SourceCommand::Stage {
            uri,
            out,
            no_detect_candidates,
            json,
        } => {
            let stage_report =
                authoring::stage_remote_source(&uri, out.as_deref(), !no_detect_candidates)?;
            if json {
                report::json(&stage_report)?;
            } else {
                report::text(&authoring::render_stage_report(&stage_report))?;
            }
        }
        SourceCommand::Map { source, out, json } => {
            let report = authoring::create_source_map_from_source(&source, &out)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_source_map_write(&report))?;
            }
        }
        SourceCommand::Query {
            map,
            handle,
            view,
            json,
        } => {
            let report = authoring::query_source_map(&map, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_source_query(&report))?;
            }
        }
        SourceCommand::Lens {
            map,
            cursor,
            limit,
            json,
        } => {
            let report = authoring::source_lens(&map, cursor, limit)?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_source_lens(&report))?;
            }
        }
        SourceCommand::Coverage { map, json } => {
            let coverage = authoring::source_coverage(&map)?;
            if json {
                report::json(&coverage)?;
            } else {
                report::text(&authoring::render_source_coverage(&coverage))?;
            }
        }
        SourceCommand::Stale { map, root, json } => {
            let report = authoring::stale_source_map(&map, root.as_deref())?;
            let ok = report.ok;
            if json {
                report::json(&report)?;
            } else {
                report::text(&authoring::render_source_stale(&report))?;
            }
            if !ok {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
