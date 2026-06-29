use super::args::{
    CapabilityCommand, Command, DurableExecutorCommand, GrammarCommand, GuideModeArg,
    InstallCommand, ProgressCommand, RouterCommand, RouterIndexCommand, SkillsCommand,
    SourceCommand, TraceCommand, VisibilityCommand, WorkspaceCommand,
};
mod deps_cmd;
mod imports_cmd;

use skillspec::{
    act, align, capability, compiler, decision, doctor, durable_lifecycle, error, grammar, guide,
    importer, install, model, parser, port_one_shot, progress, remote_source, report, router,
    router_lifecycle, run_loop, sensemake, source_map, status, trace, visibility, workspace,
    workspace_synthesizer,
};
use skillspec::{error::Result, install::HarnessTarget};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub(super) fn run(command: Command) -> Result<()> {
    match command {
        Command::Validate { path } => {
            let spec = parser::load_spec(&path)?;
            report::validation_ok(&path, &spec)?;
        }
        Command::Test { path } => {
            let spec = parser::load_spec(&path)?;
            let result = decision::run_tests(&spec);
            report::test_result(&result)?;
            if !result.failed.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Decide {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::json(&decision.decision)?;
        }
        Command::Act {
            path,
            input,
            trace_dir,
            run,
            phase,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref().or(run.as_ref()))?;
            let decision = decision::decide_with_events(&spec, &input);
            let trace = if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
                Some(trace)
            } else {
                None
            };
            let mut act_report = act::build_report_for_phase(
                &spec,
                &decision.decision,
                trace.as_ref(),
                phase.as_deref(),
            )?;
            if let Some(run) = run {
                act_report.trace = Some(act::trace_for_run(&run));
            }
            if json {
                report::json(&act_report)?;
            } else {
                report::text(&act::render(&act_report))?;
            }
        }
        Command::Plan {
            path,
            input,
            trace_dir,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            let trace = if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
                Some(trace)
            } else {
                None
            };
            let act_report = act::build_report(&spec, &decision.decision, trace.as_ref());
            if json {
                report::json(&act_report)?;
            } else {
                report::text(&act::render_plan(&act_report))?;
            }
        }
        Command::RunLoop {
            path,
            input,
            resume,
            view,
            trace_dir,
            phase,
            guide: guide_mode_arg,
            json,
        } => {
            let started = Instant::now();
            let spec = parser::load_spec(&path)?;
            if let Some(guide_mode_arg) = guide_mode_arg {
                if resume.is_none() {
                    ensure_trace_available(&spec, trace_dir.as_ref())?;
                }
                let guide_report = guide::build_report(guide::BuildOptions {
                    spec: &spec,
                    spec_path: &path,
                    input: input.as_deref(),
                    resume_run_dir: resume.as_deref(),
                    trace_dir: trace_dir.as_deref(),
                    phase_override: phase.as_deref(),
                    guide_mode: guide_mode(guide_mode_arg),
                })?;
                if json {
                    report::json(&guide_report)?;
                } else {
                    report::text(&guide::render_text(&guide_report))?;
                }
            } else {
                let input = input.ok_or_else(|| error::Error::InvalidInput {
                    message: "run-loop requires --input unless --guide --resume is used".to_owned(),
                })?;
                if resume.is_some() {
                    return Err(error::Error::InvalidInput {
                        message: "run-loop --resume requires --guide".to_owned(),
                    });
                }
                ensure_trace_available(&spec, trace_dir.as_ref())?;
                let run_loop_report = run_loop::build_report(
                    &spec,
                    &path,
                    &input,
                    view.into(),
                    trace_dir.as_deref(),
                    phase.as_deref(),
                )?;
                let elapsed = started.elapsed();
                if json {
                    report::json(&run_loop_report)?;
                } else {
                    report::text(&run_loop::render_summary(&run_loop_report, elapsed))?;
                }
            }
        }
        Command::Explain {
            path,
            input,
            trace_dir,
        } => {
            let spec = parser::load_spec(&path)?;
            ensure_trace_available(&spec, trace_dir.as_ref())?;
            let decision = decision::decide_with_events(&spec, &input);
            if let Some(trace_dir) = trace_dir {
                let trace = trace::write_decision_trace(&trace_dir, &path, &spec, &decision)?;
                report::trace_written(&trace)?;
            }
            report::explain(&decision.decision)?;
        }
        Command::Sensemake { path, view, json } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::sensemake(&spec, &path, view.into());
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_sensemake(&report))?;
            }
        }
        Command::Query {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::query(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_query(&report))?;
            }
        }
        Command::Refs {
            path,
            handle,
            view,
            json,
        } => {
            let spec = parser::load_spec(&path)?;
            let report = sensemake::refs(&spec, &path, &handle, view.into())?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&sensemake::render_refs(&report))?;
            }
        }
        Command::Doctor {
            path,
            json,
            html,
            markdown,
        } => {
            let doctor_report = doctor::inspect_target(&path)?;
            if json {
                report::json(&doctor_report)?;
            } else if html {
                report::text(&doctor::render_html(&doctor_report))?;
            } else if markdown {
                report::text(&doctor::render_markdown(&doctor_report))?;
            } else {
                report::text(&doctor::render(&doctor_report))?;
            }
        }
        Command::Status { roots, json } => {
            let status_report = status::status(status::StatusOptions { roots })?;
            if json {
                report::json(&status_report)?;
            } else {
                report::text(&status::render(&status_report))?;
            }
        }
        Command::Source { command } => match command {
            SourceCommand::Stage {
                uri,
                out,
                no_detect_candidates,
                json,
            } => {
                let stage_report = remote_source::stage_remote_source(
                    &uri,
                    out.as_deref(),
                    !no_detect_candidates,
                )?;
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
        },
        Command::Workspace { command } => match command {
            WorkspaceCommand::Map {
                source_root,
                out,
                install_slug_policy,
                json,
                summary,
            } => {
                let started = Instant::now();
                let workspace_report =
                    workspace::map_workspace(&source_root, &out, install_slug_policy.into())?;
                let elapsed = started.elapsed();
                if json {
                    report::json(&workspace_report)?;
                } else if summary {
                    report::text(&workspace::render_map_summary(&workspace_report, elapsed))?;
                } else {
                    let manifest = workspace::load_manifest(&out)?;
                    report::text(&workspace::render_map_report(&workspace_report, &manifest))?;
                }
            }
            WorkspaceCommand::Validate {
                manifest,
                json,
                summary,
            } => {
                let started = Instant::now();
                let validation_report = workspace::validate_workspace(&manifest)?;
                let elapsed = started.elapsed();
                let ok = validation_report.ok;
                if json {
                    report::json(&validation_report)?;
                } else if summary {
                    report::text(&workspace::render_validation_summary(
                        &validation_report,
                        elapsed,
                    ))?;
                } else {
                    report::text(&workspace::render_validation_report(&validation_report))?;
                }
                if !ok {
                    std::process::exit(1);
                }
            }
            WorkspaceCommand::Import {
                manifest,
                out,
                json,
                summary,
            } => {
                let started = Instant::now();
                let import_report = workspace::import_workspace(&manifest, &out)?;
                let elapsed = started.elapsed();
                let ok = import_report.ok;
                if json {
                    report::json(&import_report)?;
                } else if summary {
                    report::text(&workspace::render_import_summary(&import_report, elapsed))?;
                } else {
                    report::text(&workspace::render_import_report(&import_report))?;
                }
                if !ok {
                    std::process::exit(1);
                }
            }
            WorkspaceCommand::Converge {
                manifest,
                build_root,
                json,
                summary,
            } => {
                let started = Instant::now();
                let converge_report = workspace::converge_workspace(&manifest, &build_root)?;
                let elapsed = started.elapsed();
                let ok = converge_report.ok;
                if json {
                    report::json(&converge_report)?;
                } else if summary {
                    report::text(&workspace::render_converge_summary(
                        &converge_report,
                        elapsed,
                    ))?;
                } else {
                    report::text(&workspace::render_converge_report(&converge_report))?;
                }
                if !ok {
                    std::process::exit(1);
                }
            }
            WorkspaceCommand::Compile {
                manifest,
                build_root,
                target,
                json,
                summary,
            } => {
                let started = Instant::now();
                let compile_report =
                    workspace::compile_workspace(&manifest, &build_root, target.into())?;
                let elapsed = started.elapsed();
                let ok = compile_report.ok;
                if json {
                    report::json(&compile_report)?;
                } else if summary {
                    report::text(&workspace::render_compile_summary(&compile_report, elapsed))?;
                } else {
                    report::text(&workspace::render_compile_report(&compile_report))?;
                }
                if !ok {
                    std::process::exit(1);
                }
            }
            WorkspaceCommand::Install {
                manifest,
                build_root,
                target,
                all_detected,
                dry_run,
                retire_existing,
                install_slug_policy,
                visibility_policy,
                apply_visibility,
                visibility_manifest,
                json,
                summary,
            } => {
                let targets = target
                    .into_iter()
                    .map(HarnessTarget::from)
                    .collect::<Vec<_>>();
                let started = Instant::now();
                let install_report =
                    workspace::install_workspace(workspace::WorkspaceInstallRequest {
                        manifest_path: &manifest,
                        build_root: &build_root,
                        targets: &targets,
                        all_detected,
                        dry_run,
                        retire_existing,
                        install_slug_policy: install_slug_policy.map(Into::into),
                        visibility_policy: visibility_policy.into(),
                        apply_visibility,
                        visibility_manifest: visibility_manifest.as_deref(),
                    })?;
                let elapsed = started.elapsed();
                let ok = install_report.ok;
                if json {
                    report::json(&install_report)?;
                } else if summary {
                    report::text(&workspace::render_install_summary(&install_report, elapsed))?;
                } else {
                    report::text(&workspace::render_install_report(&install_report))?;
                }
                if !ok {
                    std::process::exit(1);
                }
            }
        },
        Command::Grammar { command } => match command {
            GrammarCommand::Sensemake { view, json } => {
                let report = grammar::sensemake(view.into());
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&grammar::render_sensemake(&report))?;
                }
            }
            GrammarCommand::Checklist { for_subject, json } => {
                let report = grammar::checklist(for_subject.into());
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&grammar::render_checklist(&report))?;
                }
            }
            GrammarCommand::Schema { json } => {
                if json {
                    report::json(&grammar::schema_json()?)?;
                } else {
                    report::text(&grammar::render_schema_summary())?;
                }
            }
        },
        Command::Trace { command } => match command {
            TraceCommand::Compact { run_dir } => {
                let trace = trace::compact(&run_dir)?;
                report::json(&trace)?;
            }
            TraceCommand::Align {
                path,
                decision_trace,
                execution_trace,
                proof_digest,
                summary,
                json,
            } => {
                let spec = parser::load_spec(&path)?;
                let report =
                    align::align_decision_trace(&spec, &path, &decision_trace, &execution_trace)?;
                let alignment_report = align::write_report_json(&decision_trace, &report)?;
                let proof_digest_path = match proof_digest {
                    Some(path) => {
                        let digest = align::build_proof_digest(&report, &alignment_report);
                        Some(align::write_proof_digest_json(&path, &digest)?)
                    }
                    None => None,
                };
                if json {
                    report::alignment_written(&alignment_report)?;
                    if let Some(path) = &proof_digest_path {
                        report::proof_digest_written(path)?;
                    }
                    report::json(&report)?;
                } else if summary {
                    report::align_summary(
                        &report,
                        &alignment_report,
                        proof_digest_path.as_deref(),
                    )?;
                } else {
                    report::alignment_written(&alignment_report)?;
                    if let Some(path) = &proof_digest_path {
                        report::proof_digest_written(path)?;
                    }
                    report::align(&report)?;
                }
                if report.has_failures() {
                    std::process::exit(1);
                }
            }
        },
        Command::Progress { command } => match command {
            ProgressCommand::Show { path, run, json } => {
                let spec = parser::load_spec(&path)?;
                let report = progress::show(&spec, &run)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&progress::render(&report))?;
                }
            }
            ProgressCommand::Record {
                run,
                event,
                phase,
                requirement,
                id,
                status,
                evidence_kind,
                evidence_ref,
                source_skill,
                message,
                json: _,
            } => {
                let event = progress::record(progress::RecordOptions {
                    run_dir: run,
                    event: event.into(),
                    phase,
                    requirement,
                    id,
                    status,
                    evidence_kind,
                    evidence_ref,
                    source_skill,
                    message,
                })?;
                report::json(&event)?;
            }
            ProgressCommand::Stats {
                run,
                workspace,
                phase,
                requirement,
                workspace_stats_json,
                workspace_stats_report,
                total_tokens,
                context_tokens,
                query_result_tokens,
                response_tokens_cached,
                saved_tokens,
                reduction_percent,
                agent_visible_tokens,
                artifact_tokens_preserved,
                avoided_tokens,
                metrics_source,
                message,
                json: _,
            } => {
                let event = progress::record_stats(progress::StatsRecordOptions {
                    run_dir: run,
                    workspace,
                    phase,
                    requirements: requirement,
                    workspace_stats_json,
                    workspace_stats_report,
                    total_tokens,
                    context_tokens,
                    query_result_tokens,
                    response_tokens_cached,
                    saved_tokens,
                    reduction_percent,
                    agent_visible_tokens,
                    artifact_tokens_preserved,
                    avoided_tokens,
                    metrics_source,
                    message,
                })?;
                report::json(&event)?;
            }
            ProgressCommand::FinalResponse {
                run,
                phase,
                requirement,
                result,
                evidence,
                alignment,
                token_savings,
                message,
                json: _,
            } => {
                let event =
                    progress::record_final_response(progress::FinalResponseRecordOptions {
                        run_dir: run,
                        phase,
                        requirements: requirement,
                        included_result: result,
                        included_evidence: evidence,
                        included_alignment: alignment,
                        included_token_savings: token_savings,
                        message,
                    })?;
                report::json(&event)?;
            }
            ProgressCommand::Batch {
                run,
                events,
                checkpoint,
                summary,
                json,
            } => {
                let report = progress::record_batch(progress::BatchRecordOptions {
                    run_dir: run,
                    events,
                    checkpoint,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&progress::render_batch_report(&report, summary))?;
                }
            }
        },
        Command::Deps { command } => deps_cmd::run(command)?,
        Command::Imports { command } => imports_cmd::run(command)?,
        Command::Compile { path, target } => {
            let spec = parser::load_spec(&path)?;
            let markdown = compiler::compile(&spec, target.into());
            std::io::stdout().lock().write_all(markdown.as_bytes())?;
        }
        Command::ImportSkill {
            path,
            out,
            source_map,
        } => {
            workspace::guard_single_skill_source(&path, "skillspec import-skill")?;
            if let Some(source_map_path) = source_map {
                let source_root = source_map::source_root_for(&path);
                let stale_report = source_map::stale(&source_map_path, Some(&source_root))?;
                if !stale_report.ok {
                    return Err(error::Error::InvalidInput {
                        message: format!(
                            "source map {} is stale for {}; rerun `skillspec source map {} --out <map-dir>` before import",
                            source_map_path.display(),
                            source_root.display(),
                            path.display()
                        ),
                    });
                }
            }
            let imported = importer::import_skill_for_output(&path, &out)?;
            parser::write_spec(&out, &imported)?;
            report::import_ok(&path, &out, &imported)?;
        }
        Command::PortOneShot {
            source,
            out,
            target,
            prove,
            force,
            run_dir,
            phase,
            requirements,
            json,
        } => {
            let started = Instant::now();
            let mut port_report = port_one_shot::run(port_one_shot::PortOneShotOptions {
                source,
                out,
                target: target.into(),
                prove,
                force,
                run_dir,
                phase,
                requirements,
            })?;
            let elapsed = started.elapsed();
            let preview = port_one_shot::render_summary(&port_report, elapsed);
            port_one_shot::record_estimated_stats(&mut port_report, elapsed, preview.len() as u64)?;
            let rendered = port_one_shot::render_summary(&port_report, elapsed);
            port_one_shot::write_report(&port_report, &rendered)?;
            if json {
                report::json(&port_report)?;
            } else {
                report::text(&rendered)?;
            }
            if prove && !port_report.ok {
                std::process::exit(1);
            }
        }
        Command::SynthesizeFromWorkspace {
            workspace,
            out,
            task,
            name,
            log_last,
            workspace_stats_report,
            workspace_log,
            workspace_meta,
            workspace_deps,
            observation_approved,
            force,
            json,
        } => {
            let synthesis = workspace_synthesizer::synthesize_from_workspace(
                workspace_synthesizer::SynthesizeOptions {
                    workspace,
                    task,
                    out,
                    name,
                    log_last,
                    workspace_stats_report,
                    workspace_log,
                    workspace_meta,
                    workspace_deps,
                    observation_approved,
                    force,
                },
            )?;
            if json {
                report::json(&synthesis)?;
            } else {
                report::text(&workspace_synthesizer::render_report(&synthesis))?;
            }
        }
        Command::Index {
            roots,
            out,
            visibility_manifest,
            json,
        } => {
            let mut report = router::index(router::IndexOptions {
                roots,
                out,
                visibility_manifest,
            })?;
            report
                .warnings
                .extend(router_lifecycle::direct_index_warnings());
            if json {
                report::json(&report)?;
            } else {
                report::text(&router::render_index(&report))?;
            }
        }
        Command::Route {
            index,
            query,
            top,
            execution_mode,
            json,
        } => {
            let report = router::route(router::RouteOptions {
                index,
                query,
                top,
                execution_mode: execution_mode.map(Into::into),
            })?;
            if json {
                report::json(&report)?;
            } else {
                report::text(&router::render_route(&report))?;
            }
        }
        Command::Skills { command } => match command {
            SkillsCommand::Audit { roots, json } => {
                let report = router::audit(&roots)?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router::render_audit(&report))?;
                }
            }
            SkillsCommand::SetVisibility {
                skill,
                visibility,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: visibility.into(),
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            SkillsCommand::Disable {
                skill,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: router::Visibility::Off,
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            SkillsCommand::Enable {
                skill,
                roots,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::set_visibility(visibility::SetVisibilityOptions {
                    roots,
                    skill,
                    visibility: router::Visibility::Implicit,
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
        },
        Command::Visibility { command } => match command {
            VisibilityCommand::Plan {
                roots,
                profile,
                json,
            } => {
                let report = visibility::plan(visibility::VisibilityPlanOptions {
                    roots,
                    profile: profile.into(),
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_plan(&report))?;
                }
            }
            VisibilityCommand::Apply {
                roots,
                profile,
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::apply(visibility::VisibilityApplyOptions {
                    roots,
                    profile: profile.into(),
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_apply(&report))?;
                }
            }
            VisibilityCommand::Restore {
                manifest,
                dry_run,
                json,
            } => {
                let report = visibility::restore(visibility::VisibilityRestoreOptions {
                    manifest,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&visibility::render_restore(&report))?;
                }
            }
        },
        Command::Router { command } => match command {
            RouterCommand::Install {
                roots,
                index,
                manifest,
                router_name,
                dry_run,
                json,
            } => {
                let report = router_lifecycle::install(router_lifecycle::RouterInstallOptions {
                    roots,
                    index,
                    manifest,
                    router_name: Some(router_name),
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_install(&report))?;
                }
            }
            RouterCommand::Uninstall {
                manifest,
                router_name,
                index,
                keep_index,
                dry_run,
                json,
            } => {
                let report =
                    router_lifecycle::uninstall(router_lifecycle::RouterUninstallOptions {
                        manifest,
                        router_name: Some(router_name),
                        index,
                        keep_index,
                        dry_run,
                    })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_uninstall(&report))?;
                }
            }
            RouterCommand::Update {
                backup_dir,
                dry_run,
                json,
            } => {
                let report = router_lifecycle::update(router_lifecycle::RouterUpdateOptions {
                    backup_dir,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_update(&report))?;
                }
            }
            RouterCommand::Enable { dry_run, json } => {
                let report =
                    router_lifecycle::enable(router_lifecycle::RouterModeOptions { dry_run })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_mode(&report))?;
                }
            }
            RouterCommand::Disable { dry_run, json } => {
                let report =
                    router_lifecycle::disable(router_lifecycle::RouterModeOptions { dry_run })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_mode(&report))?;
                }
            }
            RouterCommand::Guard { config, hook, json } => {
                let report =
                    router_lifecycle::guard(router_lifecycle::RouterGuardOptions { config, hook })?;
                if hook {
                    report::text(&router_lifecycle::render_guard_hook_json(&report)?)?;
                } else if json {
                    report::json(&report)?;
                } else {
                    report::text(&router_lifecycle::render_guard(&report))?;
                }
            }
            RouterCommand::Index { command } => match command {
                RouterIndexCommand::Refresh {
                    roots,
                    index,
                    visibility_manifest,
                    json,
                } => {
                    let report =
                        router_lifecycle::refresh(router_lifecycle::RouterRefreshOptions {
                            roots,
                            index,
                            visibility_manifest,
                        })?;
                    if json {
                        report::json(&report)?;
                    } else {
                        report::text(&router_lifecycle::render_refresh(&report))?;
                    }
                }
                RouterIndexCommand::Status {
                    roots,
                    index,
                    visibility_manifest,
                    json,
                } => {
                    let report = router::index_status(router::IndexStatusOptions {
                        roots,
                        index,
                        visibility_manifest,
                    })?;
                    if json {
                        report::json(&report)?;
                    } else {
                        report::text(&router::render_index_status(&report))?;
                    }
                }
            },
        },
        Command::DurableExecutor { command } => match command {
            DurableExecutorCommand::Install {
                source,
                target,
                all_detected,
                dry_run,
                force,
                json,
            } => {
                let targets = target
                    .into_iter()
                    .map(HarnessTarget::from)
                    .collect::<Vec<_>>();
                let report =
                    durable_lifecycle::install(durable_lifecycle::DurableInstallOptions {
                        source,
                        targets,
                        all_detected,
                        dry_run,
                        force,
                    })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&durable_lifecycle::render_install(&report))?;
                }
            }
            DurableExecutorCommand::Update {
                source,
                backup_dir,
                dry_run,
                json,
            } => {
                let report = durable_lifecycle::update(durable_lifecycle::DurableUpdateOptions {
                    source,
                    backup_dir,
                    dry_run,
                })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&durable_lifecycle::render_update(&report))?;
                }
            }
            DurableExecutorCommand::Delete { dry_run, json } => {
                let report =
                    durable_lifecycle::delete(durable_lifecycle::DurableDeleteOptions { dry_run })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&durable_lifecycle::render_delete(&report))?;
                }
            }
            DurableExecutorCommand::Enable { dry_run, json } => {
                let report =
                    durable_lifecycle::enable(durable_lifecycle::DurableModeOptions { dry_run })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&durable_lifecycle::render_mode(&report))?;
                }
            }
            DurableExecutorCommand::Disable { dry_run, json } => {
                let report =
                    durable_lifecycle::disable(durable_lifecycle::DurableModeOptions { dry_run })?;
                if json {
                    report::json(&report)?;
                } else {
                    report::text(&durable_lifecycle::render_mode(&report))?;
                }
            }
        },
        Command::Install { command } => match command {
            InstallCommand::Targets => {
                let targets = install::detect_targets()?;
                report::json(&targets)?;
            }
            InstallCommand::Skill {
                folder,
                target,
                all_detected,
                dry_run,
                force,
                retire_existing,
                name,
            } => {
                let targets = target
                    .into_iter()
                    .map(HarnessTarget::from)
                    .collect::<Vec<_>>();
                let report = install::install_skill(
                    &folder,
                    &targets,
                    all_detected,
                    dry_run,
                    force,
                    retire_existing,
                    name.as_deref(),
                )?;
                report::json(&report)?;
            }
        },
        Command::Capability { command } => match command {
            CapabilityCommand::Store => {
                report::json(&capability::store()?)?;
            }
            CapabilityCommand::Add {
                id,
                domain,
                kind,
                command,
                adapter,
                script,
                provides,
                alias,
                priority,
                preferred_for,
                avoid_for,
                ties,
                auth_env,
                external_service,
                may_cost_money,
                evidence_command,
                suggested_skill_id,
            } => {
                let report = capability::add(capability::AddOptions {
                    id,
                    domain,
                    kind,
                    command,
                    adapter,
                    script,
                    provides,
                    aliases: alias,
                    priority,
                    preferred_for,
                    avoid_for,
                    ties,
                    auth_env,
                    external_service,
                    may_cost_money,
                    evidence_command,
                    suggested_skill_id,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Update {
                id,
                domain,
                kind,
                command,
                clear_command,
                adapter,
                clear_adapter,
                script,
                clear_script,
                add_provides,
                remove_provides,
                add_alias,
                remove_alias,
                priority,
                clear_priority,
                add_preferred_for,
                remove_preferred_for,
                add_avoid_for,
                remove_avoid_for,
                add_tie,
                remove_tie,
                add_auth_env,
                remove_auth_env,
                external_service,
                may_cost_money,
                add_evidence_command,
                remove_evidence_command,
                suggested_skill_id,
                clear_suggested_skill_id,
                mark_unverified,
                mark_failed,
            } => {
                let verification_status = if mark_failed {
                    Some(capability::VerificationStatus::Failed)
                } else if mark_unverified {
                    Some(capability::VerificationStatus::Unverified)
                } else {
                    None
                };
                let report = capability::update(capability::UpdateOptions {
                    id,
                    domain,
                    kind,
                    command,
                    clear_command,
                    adapter,
                    clear_adapter,
                    script,
                    clear_script,
                    add_provides,
                    remove_provides,
                    add_alias,
                    remove_alias,
                    priority,
                    clear_priority,
                    add_preferred_for,
                    remove_preferred_for,
                    add_avoid_for,
                    remove_avoid_for,
                    add_ties: add_tie,
                    remove_tie,
                    add_auth_env,
                    remove_auth_env,
                    external_service,
                    may_cost_money,
                    add_evidence_command,
                    remove_evidence_command,
                    suggested_skill_id,
                    clear_suggested_skill_id,
                    verification_status,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::List { domain } => {
                report::json(&capability::list(domain.as_deref())?)?;
            }
            CapabilityCommand::Search {
                capability: capability_id,
                domain,
                explain: _,
                json: _,
                local_only,
                preferred_seed,
            } => {
                let report = capability::search(capability::SearchOptions {
                    capability: capability_id,
                    domain,
                    local_only,
                    preferred_seed,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Inspect {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::inspect(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Verify {
                id,
                domain,
                json: _,
            } => {
                report::json(&capability::verify(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Prefer {
                id,
                domain,
                for_capability,
                priority,
            } => {
                let report = capability::prefer(capability::PreferOptions {
                    id,
                    domain,
                    for_capability,
                    priority,
                })?;
                report::json(&report)?;
            }
            CapabilityCommand::Remove { id, domain } => {
                report::json(&capability::remove(&id, domain.as_deref())?)?;
            }
            CapabilityCommand::Scan => {
                report::json(&capability::scan()?)?;
            }
        },
    }

    Ok(())
}

fn ensure_trace_available(spec: &model::SkillSpec, trace_dir: Option<&PathBuf>) -> Result<()> {
    if spec
        .trace
        .as_ref()
        .is_some_and(|trace| trace.required && trace_dir.is_none())
    {
        return Err(error::Error::InvalidInput {
            message: "trace.required is true; pass --trace-dir or use a spec that does not require tracing"
                .to_owned(),
        });
    }
    Ok(())
}

fn guide_mode(mode: GuideModeArg) -> guide::GuideMode {
    match mode {
        GuideModeArg::Agent => guide::GuideMode::Agent,
        GuideModeArg::Full => guide::GuideMode::Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_required_requires_trace_dir() {
        let yaml = r#"
schema: skillspec/v0
id: trace.required
title: Trace Required
description: Requires trace output.
routes:
  - id: local
    label: Local
trace:
  mode: event_log
  required: true
tests:
  - name: route assertion
    input: run this
    expect:
      route: local
"#;
        let spec = serde_yaml::from_str::<model::SkillSpec>(yaml).unwrap();
        let trace_dir = PathBuf::from(".skillspec/traces");

        assert!(ensure_trace_available(&spec, None).is_err());
        assert!(ensure_trace_available(&spec, Some(&trace_dir)).is_ok());
    }
}
