use super::command_inference::{dependency_id_for_tool, ObservedCommand};
use serde::Serialize;
use serde_yaml::Value as YamlValue;
use skillspec_core::import_dependency_ledger;
use skillspec_core::model::{
    Activation, Artifact, ArtifactKind, CommandRequires, CommandTemplate, Dependency,
    DependencyCheck, DependencyKind, DependencyPermission, Elicitation, ElicitationChoice,
    ElicitationCondition, Entry, ExecutionPhase, ExecutionPlan, ExecutionPlanMode, Expectation,
    Proof, Recipe, RecipeRequires, RecipeStep, RecipeStepAsk, RecipeStepNote, RecipeStepRunCommand,
    Route, RouteId, Rule, RuleId, SafetyClass, ScenarioTest, SkillSpec, ToolBoundary,
    ToolBoundaryDefault, TraceConfig, TraceEventKind, TraceMode,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
struct CliSurface {
    binary: String,
    dependency_id: String,
    label: String,
}

pub(super) fn build_spec(
    skill_id: &str,
    title: &str,
    command_candidates: &[ObservedCommand],
) -> SkillSpec {
    let cli = primary_cli(command_candidates);
    let cli_binary = cli.binary.as_str();
    let cli_dependency = cli.dependency_id.clone();
    let cli_label = cli.label.as_str();
    let has_auth_status_error = command_candidates.iter().any(|command| {
        command
            .template
            .to_ascii_lowercase()
            .contains(&format!("{} auth status", cli_binary).to_ascii_lowercase())
    });
    let has_missing_target_error = command_candidates.iter().any(|command| {
        let template = command.template.to_ascii_lowercase();
        template.contains("enrich run") && !template.contains("--target")
    });

    let mut dependencies = BTreeMap::new();
    dependencies.insert(
        cli_dependency.clone(),
        Dependency {
            kind: DependencyKind::Cli,
            description: Some(format!(
                "{cli_label} used for authentication checks and profile enrichment."
            )),
            command: Some(cli_binary.to_owned()),
            path: None,
            env: None,
            check: Some(DependencyCheck {
                command: Some(cli_binary.to_owned()),
                path: None,
                env: None,
            }),
            permission: Some(DependencyPermission {
                required: true,
                reason: Some(
                    "Enrichment can perform authenticated network requests and may write an output file."
                        .to_owned(),
                ),
                safety: Some(SafetyClass::NetworkRead),
            }),
            provision: None,
        },
    );
    dependencies.insert(
        import_dependency_ledger::DEPENDENCY_LEDGER_ID.to_owned(),
        import_dependency_ledger::dependency(
            "Generated dependency ledger preserving dependency evidence from synthesized CLI material.",
        ),
    );

    let mut commands = BTreeMap::new();
    commands.insert(
        "cli_version".to_owned(),
        CommandTemplate {
            description: Some("Verify the selected CLI binary.".to_owned()),
            template: format!("{cli_binary} --version"),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_auth".to_owned(),
        CommandTemplate {
            description: Some(format!(
                "Check CLI authentication. Use `{cli_binary} auth`; do not append a status subcommand."
            )),
            template: format!("{cli_binary} auth"),
            safety: Some(SafetyClass::NetworkRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_enrich_dry_run".to_owned(),
        CommandTemplate {
            description: Some(
                "Validate the fully parameterized command shape without running enrichment.".to_owned(),
            ),
            template: format!(
                "{cli_binary} enrich run --data '<people_json>' --target <csv_output> --intent '<profile_enrichment_intent>' --processor <processor> --json --dry-run"
            ),
            safety: Some(SafetyClass::LocalRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );
    commands.insert(
        "cli_enrich_run".to_owned(),
        CommandTemplate {
            description: Some(
                "Run the approved enrichment command with the same typed inputs used in the dry run."
                    .to_owned(),
            ),
            template: format!(
                "{cli_binary} enrich run --data '<people_json>' --target <csv_output> --intent '<profile_enrichment_intent>' --processor <processor> --json"
            ),
            safety: Some(SafetyClass::NetworkRead),
            requires: CommandRequires {
                dependencies: vec![cli_dependency.clone()],
                ..CommandRequires::default()
            },
            parse: BTreeMap::new(),
            success_when: BTreeMap::new(),
        },
    );

    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        import_dependency_ledger::DEPENDENCY_LEDGER_ID.to_owned(),
        import_dependency_ledger::artifact(
            "Generated dependency ledger preserving dependency evidence from synthesized CLI material.",
        ),
    );
    artifacts.insert(
        "enrichment_json".to_owned(),
        Artifact {
            kind: ArtifactKind::Report,
            description: Some(format!(
                "JSON emitted by the {cli_label} enrichment command."
            )),
            path: None,
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );
    artifacts.insert(
        "enrichment_csv".to_owned(),
        Artifact {
            kind: ArtifactKind::File,
            description: Some(format!(
                "User-selected CSV output path produced by the {cli_label} enrichment command."
            )),
            path: None,
            schema: None,
            produced_by: Vec::new(),
            consumed_by: Vec::new(),
        },
    );

    let mut elicitations = BTreeMap::new();
    elicitations.insert(
        "provide_profile_enrichment_inputs".to_owned(),
        Elicitation {
            question: format!(
                "Provide the people or entities, enrichment intent, processor, and output path for this {cli_label} run."
            ),
            required_when: vec![ElicitationCondition {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                missing: None,
                predicate: None,
            }],
            choices: vec![
                ElicitationChoice {
                    id: "use_supplied_structured_inputs".to_owned(),
                    label: "Use supplied inputs".to_owned(),
                    description: Some("Continue only when people_json, profile_enrichment_intent, processor, and csv_output are already present in the user request or attached context.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
                ElicitationChoice {
                    id: "ask_for_missing_inputs".to_owned(),
                    label: "Ask for missing inputs".to_owned(),
                    description: Some(
                        "Stop and ask for any missing typed values before constructing commands."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
                ElicitationChoice {
                    id: "draft_only".to_owned(),
                    label: "Draft only".to_owned(),
                    description: Some(
                        "Do not execute; return the parameter contract and command templates only."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
            ],
            default: Some("ask_for_missing_inputs".to_owned()),
            max_choices: None,
        },
    );
    elicitations.insert(
        "approve_cli_execution".to_owned(),
        Elicitation {
            question: format!(
                "Approve running the parameterized {cli_label} enrichment after the dry run passes?"
            ),
            required_when: vec![ElicitationCondition {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                missing: None,
                predicate: None,
            }],
            choices: vec![
                ElicitationChoice {
                    id: "approve_run".to_owned(),
                    label: "Approve run".to_owned(),
                    description: Some(
                        "Run the reviewed command using the supplied typed inputs.".to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::NetworkRead),
                },
                ElicitationChoice {
                    id: "dry_run_only".to_owned(),
                    label: "Dry run only".to_owned(),
                    description: Some(
                        "Stop after command-shape validation without performing enrichment."
                            .to_owned(),
                    ),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::LocalRead),
                },
                ElicitationChoice {
                    id: "revise_inputs".to_owned(),
                    label: "Revise inputs".to_owned(),
                    description: Some("Return to input collection before execution.".to_owned()),
                    sets: BTreeMap::new(),
                    route: None,
                    next: None,
                    safety: Some(SafetyClass::ReadOnly),
                },
            ],
            default: Some("dry_run_only".to_owned()),
            max_choices: None,
        },
    );

    let mut recipes = BTreeMap::new();
    recipes.insert(
        "run_profile_enrichment".to_owned(),
        Recipe {
            description: Some(format!("Ordered {cli_label} profile-enrichment workflow.")),
            ordered: true,
            requires: RecipeRequires {
                dependencies: vec![cli_dependency.clone()],
                ..RecipeRequires::default()
            },
            steps: vec![
                RecipeStep::Ask(RecipeStepAsk {
                    ask: "provide_profile_enrichment_inputs".to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Required typed inputs are people_json array, profile_enrichment_intent string, processor enum, csv_output path, and summary_output preference."
                        .to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Do not reuse example people or preserve one-off literal names from earlier runs."
                        .to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_version".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_auth".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_enrich_dry_run".to_owned(),
                }),
                RecipeStep::Ask(RecipeStepAsk {
                    ask: "approve_cli_execution".to_owned(),
                }),
                RecipeStep::RunCommand(RecipeStepRunCommand {
                    run_command: "cli_enrich_run".to_owned(),
                }),
                RecipeStep::Note(RecipeStepNote {
                    note: "Summarize only professional/public information, flag ambiguity, and state when source URLs are absent."
                        .to_owned(),
                }),
            ],
        },
    );

    let route = Route {
        id: RouteId("profile_enrichment_cli".to_owned()),
        label: "Enrich public profiles with selected CLI".to_owned(),
        rank: Some(1),
        description: Some(format!("Run a parameterized {cli_label} enrichment workflow using user-supplied people, enrichment intent, processor, and output path.")),
        checks: Vec::new(),
        handoff: None,
        execution_plan: Some(ExecutionPlan {
            mode: ExecutionPlanMode::Ordered,
            reason: Some("Profile enrichment needs typed inputs, CLI readiness, a dry-run check, approved execution, and a privacy-aware result summary.".to_owned()),
            phases: vec![
                ExecutionPhase {
                    id: "collect_inputs".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Collect or confirm typed inputs before constructing any command.".to_owned()),
                    requires: vec![
                        "collect_typed_profile_inputs".to_owned(),
                        "validate_parameter_contract".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "hardcoded_people_values".to_owned(),
                        "reuse_example_people".to_owned(),
                        "execute_before_required_inputs".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "preflight_cli".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Verify the selected CLI binary and authentication state.".to_owned()),
                    requires: vec!["check_cli_version".to_owned(), "check_cli_auth".to_owned()],
                    checks: Vec::new(),
                    forbid: vec![
                        "exact_parallel_binary_assumption".to_owned(),
                        "use_auth_status_subcommand".to_owned(),
                        "use_cli_without_auth_check".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "dry_run".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Dry-run the fully parameterized enrichment command and fix command-shape errors before network execution.".to_owned()),
                    requires: vec![
                        "run_cli_enrich_dry_run".to_owned(),
                        "review_dry_run_output".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "omit_target_option".to_owned(),
                        "run_without_dry_run".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "execute_enrichment".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Run the approved CLI enrichment command and capture JSON plus the requested output file.".to_owned()),
                    requires: vec![
                        "approve_cli_execution".to_owned(),
                        "run_cli_enrich".to_owned(),
                        "capture_cli_stdout".to_owned(),
                        "capture_output_file".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "execute_before_approval".to_owned(),
                        "mutate_input_values_during_execution".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
                ExecutionPhase {
                    id: "summarize_results".to_owned(),
                    owner_skill: skill_id.to_owned(),
                    route: None,
                    description: Some("Summarize the final output with privacy, ambiguity, and source-claim guardrails.".to_owned()),
                    requires: vec![
                        "produce_profile_enrichment_summary".to_owned(),
                        "report_cli_errors_and_caveats".to_owned(),
                    ],
                    checks: Vec::new(),
                    forbid: vec![
                        "include_unnecessary_address_phone_pii".to_owned(),
                        "merge_ambiguous_same_name_identities".to_owned(),
                        "claim_source_urls_when_absent".to_owned(),
                    ],
                    handoff: None,
                    jumps: Vec::new(),
                    tool_boundary: None,
                },
            ],
        }),
        tool_boundary: None,
    };

    let rule = Rule {
        id: RuleId("route_parallel_profile_enrichment".to_owned()),
        when: skillspec_core::model::Predicate {
            user_says_any: activation_terms(cli_binary),
            ..skillspec_core::model::Predicate::default()
        },
        prefer: Some(RouteId("profile_enrichment_cli".to_owned())),
        route_order: Vec::new(),
        forbid: vec![
            "hardcoded_people_values".to_owned(),
            "reuse_example_people".to_owned(),
            "execute_before_required_inputs".to_owned(),
            "exact_parallel_binary_assumption".to_owned(),
            "use_auth_status_subcommand".to_owned(),
            "omit_target_option".to_owned(),
            "run_without_dry_run".to_owned(),
            "include_unnecessary_address_phone_pii".to_owned(),
            "merge_ambiguous_same_name_identities".to_owned(),
            "claim_source_urls_when_absent".to_owned(),
        ],
        allow: BTreeMap::new(),
        elicit: vec![
            "provide_profile_enrichment_inputs".to_owned(),
            "approve_cli_execution".to_owned(),
        ],
        after_success: vec!["produce_profile_enrichment_summary".to_owned()],
        reason: Some("CLI profile enrichment must be parameterized from user-provided inputs and must preserve CLI error lessons as hard guards.".to_owned()),
    };

    let mut closures = BTreeMap::new();
    for (id, description) in [
        (
            "collect_typed_profile_inputs",
            "Ensure people_json is an array of objects, profile_enrichment_intent is a string, processor is one of the supported processors, csv_output is a writable path, and summary_output states the desired response format.",
        ),
        (
            "validate_parameter_contract",
            "Reject hardcoded or example people values; every run must receive fresh typed inputs or ask the user for them.",
        ),
        (
            "check_cli_version",
            "Prove the selected CLI responds to --version before use.",
        ),
        (
            "check_cli_auth",
            "Prove authentication with the detected CLI auth command; auth status subcommands are forbidden when they failed during command exploration.",
        ),
        (
            "run_cli_enrich_dry_run",
            "Run the dry-run command with --target present; missing --target is a known invalid command shape.",
        ),
        (
            "review_dry_run_output",
            "Confirm the dry run reports the expected row count, source columns, target path, processor, and intent before execution.",
        ),
        (
            "approve_cli_execution",
            "Ask for approval after dry-run success and before network enrichment.",
        ),
        (
            "run_cli_enrich",
            "Execute the same parameterized command shape that passed dry-run, without mutating inputs.",
        ),
        (
            "capture_cli_stdout",
            "Preserve JSON stdout or an equivalent response artifact for summary.",
        ),
        (
            "capture_output_file",
            "Preserve the requested CSV output file path and hash when available.",
        ),
        (
            "produce_profile_enrichment_summary",
            "Summarize final output, report command caveats, omit unnecessary address/phone PII, avoid identity merges, and do not claim source URLs unless the CLI output includes them.",
        ),
        (
            "report_cli_errors_and_caveats",
            "Report invalid command forms encountered during setup, including auth status misuse and missing target output.",
        ),
    ] {
        closures.insert(id.to_owned(), yaml_description(description));
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("parameter_contract".to_owned(), parameter_contract());
    metadata.insert(
        "learned_cli_error_guards".to_owned(),
        learned_cli_error_guards(cli_binary, has_auth_status_error, has_missing_target_error),
    );
    metadata.insert("coverage_matrix".to_owned(), coverage_matrix());
    metadata.insert("contract_quality".to_owned(), contract_quality());
    metadata.insert(
        "command_candidate_count".to_owned(),
        yaml(command_candidates.len()),
    );

    SkillSpec {
        schema: "skillspec/v0".to_owned(),
        id: skill_id.to_owned(),
        title: title.to_owned(),
        description: "Reusable CLI workflow for enriching public professional profiles from typed, user-provided inputs."
            .to_owned(),
        activation: Some(Activation {
            summary: "Use when the user wants to enrich public professional profiles with a detected CLI from a list of people or entities."
                .to_owned(),
            keywords: activation_terms(cli_binary),
            priority: Some("domain".to_owned()),
        }),
        applies_when: vec![yaml(serde_json::json!({
            "user_intent": [
                "enrich public professional profiles with a detected CLI",
                "turn a typed list of people into public professional profile summaries",
                "run CLI enrichment with explicit people, intent, processor, and output parameters",
            ]
        }))],
        entry: Some(Entry {
            prompt: "Collect typed profile-enrichment inputs, validate that no example values are reused, dry-run the detected CLI command, run only the approved command, and summarize public professional information with privacy and ambiguity guardrails."
                .to_owned(),
            decision_required: true,
            supersedes_skills: Vec::new(),
            forbid_before_decision: vec![
                "hardcoded_people_values".to_owned(),
                "reuse_example_people".to_owned(),
                "execute_before_required_inputs".to_owned(),
                "use_cli_without_auth_check".to_owned(),
                "run_without_dry_run".to_owned(),
            ],
            tool_boundary: Some(ToolBoundary {
                default: Some(ToolBoundaryDefault::Deny),
                allow: vec![
                    "skillspec_cli".to_owned(),
                    "local_skill_files".to_owned(),
                    "declared_commands_dependencies_imports_resources".to_owned(),
                ],
                forbid: Vec::new(),
                permission_required_for: vec![
                    "cli_execution".to_owned(),
                    "network_access".to_owned(),
                    "local_file_write".to_owned(),
                    "any_unlisted_tool".to_owned(),
                ],
            }),
        }),
        routes: vec![route],
        rules: vec![rule],
        states: BTreeMap::new(),
        elicitations,
        trace: Some(TraceConfig {
            mode: TraceMode::EventLog,
            required: true,
            record: vec![
                TraceEventKind::InputReceived,
                TraceEventKind::RuleMatched,
                TraceEventKind::RouteSelected,
                TraceEventKind::ElicitationRequested,
                TraceEventKind::OutcomeRecorded,
            ],
        }),
        dependencies,
        imports: BTreeMap::new(),
        resources: BTreeMap::new(),
        code: BTreeMap::new(),
        artifacts,
        recipes,
        commands,
        snippets: BTreeMap::new(),
        closures,
        proof: Some(Proof {
            metrics: vec![
                "typed inputs collected before command construction".to_owned(),
                "detected CLI readiness checked before enrichment".to_owned(),
                "dry-run command succeeds before enrichment".to_owned(),
                "execution approval recorded before network run".to_owned(),
            ],
        }),
        tests: profile_enrichment_tests(cli_binary),
        review_required: review_notes(command_candidates),
        metadata,
    }
}

fn review_notes(command_candidates: &[ObservedCommand]) -> Vec<String> {
    let mut notes = vec![
        "Execute only after fresh typed inputs are supplied and the dry run succeeds.".to_owned(),
        "Do not install until the user approves the dependency and execution surface.".to_owned(),
        "Review deps.toml and preserve dependency authority, local status, install risk, and degraded proof impact before proof or install.".to_owned(),
        "Add narrower source-oriented commands if future runs require source URLs in the final summary."
            .to_owned(),
    ];
    if command_candidates.is_empty() {
        notes.push("No stable CLI command candidates were inferred; keep the generated workflow draft-only until a command transcript is reviewed.".to_owned());
    }
    notes
}

fn primary_cli(command_candidates: &[ObservedCommand]) -> CliSurface {
    let mut counts: BTreeMap<String, (usize, String)> = BTreeMap::new();
    for command in command_candidates {
        let Some(tool) = command.tool.as_deref() else {
            continue;
        };
        let dependency_id = dependency_id_for_tool(tool);
        if dependency_id.is_empty() {
            continue;
        }
        let entry = counts.entry(dependency_id).or_insert((0, tool.to_owned()));
        entry.0 += 1;
    }

    let (dependency_id, (_, binary)) = counts
        .into_iter()
        .max_by(|(left_id, (left_count, _)), (right_id, (right_count, _))| {
            left_count
                .cmp(right_count)
                .then_with(|| right_id.cmp(left_id))
        })
        .unwrap_or_else(|| ("cli".to_owned(), (0, "cli".to_owned())));
    let label = if binary == "cli" {
        "selected CLI".to_owned()
    } else {
        format!("`{binary}` CLI")
    };
    CliSurface {
        binary,
        dependency_id,
        label,
    }
}

fn profile_enrichment_tests(cli_binary: &str) -> Vec<ScenarioTest> {
    vec![
        ScenarioTest {
            name: "generic profile enrichment routes to cli workflow".to_owned(),
            input: "enrich public professional profiles with a detected CLI".to_owned(),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                elicit: vec![
                    "provide_profile_enrichment_inputs".to_owned(),
                    "approve_cli_execution".to_owned(),
                ],
                forbid: vec![
                    "hardcoded_people_values".to_owned(),
                    "reuse_example_people".to_owned(),
                    "omit_target_option".to_owned(),
                    "run_without_dry_run".to_owned(),
                    "claim_source_urls_when_absent".to_owned(),
                ],
                after_success: vec!["produce_profile_enrichment_summary".to_owned()],
                ..Expectation::default()
            },
        },
        ScenarioTest {
            name: "incomplete request asks for typed inputs".to_owned(),
            input: "use cli to enrich these people".to_owned(),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                elicit: vec!["provide_profile_enrichment_inputs".to_owned()],
                forbid: vec![
                    "execute_before_required_inputs".to_owned(),
                    "hardcoded_people_values".to_owned(),
                ],
                ..Expectation::default()
            },
        },
        ScenarioTest {
            name: "command lessons are hard forbids".to_owned(),
            input: format!("{cli_binary} enrich for public professional profile summaries"),
            expect: Expectation {
                route: Some(RouteId("profile_enrichment_cli".to_owned())),
                matched_rules: vec![RuleId("route_parallel_profile_enrichment".to_owned())],
                forbid: vec![
                    "use_auth_status_subcommand".to_owned(),
                    "omit_target_option".to_owned(),
                    "exact_parallel_binary_assumption".to_owned(),
                ],
                ..Expectation::default()
            },
        },
    ]
}

fn activation_terms(cli_binary: &str) -> Vec<String> {
    let mut terms = [
        "cli",
        "detected cli",
        "profile enrichment",
        "enrich profiles",
        "public professional profiles",
        "people enrichment",
        "professional summary",
        "source-oriented profile lookup",
        "cli profile enrichment",
        "enrich public professional profiles with cli",
        "enrich public profiles",
        "enrich professional profiles",
        "use cli",
    ]
    .iter()
    .map(|term| (*term).to_owned())
    .collect::<BTreeSet<_>>();
    if !cli_binary.trim().is_empty() && cli_binary != "cli" {
        terms.insert(cli_binary.to_ascii_lowercase());
        terms.insert(cli_binary.replace('-', " ").to_ascii_lowercase());
        terms.insert(format!("{} enrich", cli_binary.to_ascii_lowercase()));
    }
    terms.into_iter().collect()
}

fn yaml_description(description: &str) -> YamlValue {
    yaml(serde_json::json!({ "description": description }))
}

fn parameter_contract() -> YamlValue {
    yaml(serde_json::json!({
        "people_json": {
            "type": "array<object>",
            "required": true,
            "description": "Fresh user-supplied people or entity records for this run."
        },
        "profile_enrichment_intent": {
            "type": "string",
            "required": true,
            "description": "The public professional facts to retrieve and the ambiguity policy to apply."
        },
        "processor": {
            "type": "enum",
            "required": true,
            "allowed_values": [
                "lite", "lite-fast", "base", "base-fast", "core", "core-fast", "pro", "pro-fast",
                "ultra", "ultra-fast", "ultra2x", "ultra2x-fast", "ultra4x", "ultra4x-fast",
                "ultra8x", "ultra8x-fast"
            ]
        },
        "csv_output": {
            "type": "path",
            "required": true,
            "description": "Output CSV path passed through --target."
        },
        "summary_output": {
            "type": "enum",
            "required": true,
            "allowed_values": ["concise", "table", "json"]
        }
    }))
}

fn learned_cli_error_guards(
    cli_binary: &str,
    has_auth_status_error: bool,
    has_missing_target_error: bool,
) -> YamlValue {
    let mut guards = Vec::new();
    if has_auth_status_error {
        guards.push(serde_json::json!({
            "invalid_form": format!("{cli_binary} auth status"),
            "guard": "use_auth_status_subcommand",
            "correction": format!("{cli_binary} auth")
        }));
    }
    if has_missing_target_error {
        guards.push(serde_json::json!({
            "invalid_form": "enrich run without --target",
            "guard": "omit_target_option",
            "correction": "include --target <csv_output>"
        }));
    }
    if guards.is_empty() {
        guards.push(serde_json::json!({
            "invalid_form": "unreviewed literal command reuse",
            "guard": "hardcoded_people_values",
            "correction": "collect typed inputs and render placeholders before execution"
        }));
    }
    yaml(guards)
}

fn coverage_matrix() -> YamlValue {
    yaml(serde_json::json!([
        {
            "prose_span": "CLI command behavior",
            "obligation": "Preserve the actual CLI command shape and setup errors as reusable guards.",
            "skillspec_construct": "commands.cli_auth, commands.cli_enrich_dry_run, commands.cli_enrich_run, closures.report_cli_errors_and_caveats",
            "confidence": "high",
            "status": "reviewed",
            "review_note": "The invalid auth-status and missing-target forms become explicit forbids when present in the command transcript."
        },
        {
            "prose_span": "parameterization",
            "obligation": "Require fresh typed inputs and forbid hardcoded people values.",
            "skillspec_construct": "elicitations.provide_profile_enrichment_inputs, metadata.parameter_contract, closures.validate_parameter_contract",
            "confidence": "high",
            "status": "reviewed",
            "review_note": "The command templates use placeholders only."
        },
        {
            "prose_span": "final output",
            "obligation": "Summarize public professional information while preserving ambiguity and source-claim limits.",
            "skillspec_construct": "closures.produce_profile_enrichment_summary, rules.route_parallel_profile_enrichment.forbid",
            "confidence": "medium",
            "status": "reviewed",
            "review_note": "The summary must report CLI caveats instead of inventing unavailable source URLs."
        }
    ]))
}

fn contract_quality() -> YamlValue {
    yaml(serde_json::json!({
        "activation": "strong",
        "dependencies": "good",
        "route_coverage": "focused",
        "execution_evidence": "command_shape_reviewed",
        "tests": "reviewed_behavior_tests",
        "hallucination_risk": "low"
    }))
}

fn yaml<T: Serialize>(value: T) -> YamlValue {
    serde_yaml::to_value(value).expect("synthesized metadata must serialize")
}
