use crate::model::{
    CommandTemplate, Elicitation, ElicitationChoice, Predicate, Route, Rule, SafetyClass,
    ScenarioTest, SkillSpec, State, TraceEventKind,
};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Copy, Debug)]
pub enum Target {
    CodexSkill,
    ClaudeSkill,
    Markdown,
}

pub fn compile(spec: &SkillSpec, target: Target) -> String {
    let mut output = String::new();
    write_frontmatter(&mut output, spec, target);
    write_overview(&mut output, spec, target);
    write_runtime_contract(&mut output);
    write_entry(&mut output, spec);
    write_activation(&mut output, spec);
    write_routes(&mut output, spec);
    write_rules(&mut output, spec);
    write_elicitations(&mut output, spec);
    write_trace(&mut output, spec);
    write_states(&mut output, spec);
    write_commands(&mut output, spec);
    write_snippets(&mut output, spec);
    write_closures(&mut output, spec);
    write_tests(&mut output, spec);
    write_proof(&mut output, spec);
    write_review_required(&mut output, spec);
    write_runtime_commands(&mut output);
    output
}

fn write_frontmatter(output: &mut String, spec: &SkillSpec, target: Target) {
    match target {
        Target::CodexSkill | Target::ClaudeSkill => {
            let _ = writeln!(output, "---");
            let _ = writeln!(output, "name: {}", skill_name(&spec.id));
            let _ = writeln!(output, "description: {:?}", spec.description);
            let _ = writeln!(output, "---");
            output.push('\n');
        }
        Target::Markdown => {}
    }
}

fn write_overview(output: &mut String, spec: &SkillSpec, target: Target) {
    let _ = writeln!(output, "# {}", spec.title);
    output.push('\n');
    let _ = writeln!(output, "{}", spec.description);
    output.push('\n');
    match target {
        Target::CodexSkill | Target::ClaudeSkill => {
            output.push_str(
                "This skill was generated from a SkillSpec. Use this document as the loaded harness guidance, and treat the referenced structured decisions as the behavioral contract.\n\n",
            );
        }
        Target::Markdown => {
            output.push_str(
                "This document is a complete Markdown rendering of the SkillSpec behavioral contract.\n\n",
            );
        }
    }
}

fn write_runtime_contract(output: &mut String) {
    output.push_str("## Runtime Contract\n\n");
    output.push_str("- Read this generated skill for orientation and immediate rules.\n");
    output.push_str("- Treat routes, rules, states, commands, tests, and review notes below as authoritative.\n");
    output.push_str("- Rules beat prose when there is tension.\n");
    output.push_str("- `forbid` entries are hard negative steering, not suggestions.\n");
    output.push_str("- `elicit` entries require bounded user questions before guessing.\n");
    output.push_str("- Use the scenario tests as examples of expected behavior.\n");
    output.push_str("- When the `skillspec` CLI is available, prefer `skillspec decide` or `skillspec explain` over manual interpretation.\n");
    output.push_str("- When invoking `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/rote-shell-spec`, `$rote-shell-spec`, or `/my-skill` before setting `--input`.\n");
    output.push_str("- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.\n");
    output.push_str("- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.\n");
    output.push_str("- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.\n");
    output.push_str("- After `skillspec decide` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.\n\n");
}

fn write_entry(output: &mut String, spec: &SkillSpec) {
    if let Some(entry) = &spec.entry {
        output.push_str("## Entry\n\n");
        let _ = writeln!(output, "Prompt: {}", entry.prompt);
        output.push('\n');
    }
}

fn write_activation(output: &mut String, spec: &SkillSpec) {
    if spec.applies_when.is_empty() {
        return;
    }
    output.push_str("## Applies When\n\n");
    for hint in &spec.applies_when {
        write_yaml_block(output, hint);
    }
    output.push('\n');
}

fn write_routes(output: &mut String, spec: &SkillSpec) {
    if spec.routes.is_empty() {
        return;
    }
    output.push_str("## Routes\n\n");
    output.push_str(
        "Try lower-rank routes first unless matching rules override the route or route order.\n\n",
    );
    let mut routes = spec.routes.iter().collect::<Vec<_>>();
    routes.sort_by_key(|route| route.rank.unwrap_or(i64::MAX));
    for route in routes {
        write_route(output, route);
    }
}

fn write_route(output: &mut String, route: &Route) {
    let rank = route
        .rank
        .map(|rank| rank.to_string())
        .unwrap_or_else(|| "unranked".to_owned());
    let _ = writeln!(output, "### `{}`", route.id.0);
    let _ = writeln!(output, "- label: {}", route.label);
    let _ = writeln!(output, "- rank: {rank}");
    if let Some(description) = &route.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if !route.checks.is_empty() {
        let _ = writeln!(output, "- checks: {}", route.checks.join(", "));
    }
    output.push('\n');
}

fn write_rules(output: &mut String, spec: &SkillSpec) {
    if spec.rules.is_empty() {
        return;
    }
    output.push_str("## Rules\n\n");
    output.push_str("Evaluate rules in order. A matching rule may choose a route, replace route order, forbid substitutions, allow narrow fallbacks, request bounded elicitation, and schedule post-success actions.\n\n");
    for rule in &spec.rules {
        write_rule(output, rule);
    }
}

fn write_rule(output: &mut String, rule: &Rule) {
    let _ = writeln!(output, "### `{}`", rule.id.0);
    write_predicate(output, &rule.when);
    if let Some(route) = &rule.prefer {
        let _ = writeln!(output, "- prefer: `{}`", route.0);
    }
    if !rule.route_order.is_empty() {
        let _ = writeln!(
            output,
            "- route_order: {}",
            rule.route_order
                .iter()
                .map(|route| format!("`{}`", route.0))
                .collect::<Vec<_>>()
                .join(" -> ")
        );
    }
    if !rule.forbid.is_empty() {
        let _ = writeln!(output, "- forbid: {}", code_list(&rule.forbid));
    }
    if !rule.allow.is_empty() {
        output.push_str("- allow:\n");
        write_yaml_map(output, &rule.allow, 2);
    }
    if !rule.elicit.is_empty() {
        let _ = writeln!(output, "- elicit: {}", code_list(&rule.elicit));
    }
    if !rule.after_success.is_empty() {
        let _ = writeln!(
            output,
            "- after_success: {}",
            code_list(&rule.after_success)
        );
    }
    if let Some(reason) = &rule.reason {
        let _ = writeln!(output, "- reason: {reason}");
    }
    output.push('\n');
}

fn write_elicitations(output: &mut String, spec: &SkillSpec) {
    if spec.elicitations.is_empty() {
        return;
    }
    output.push_str("## Elicitations\n\n");
    output.push_str("Use elicitations for bounded, high-signal questions. Do not replace them with open-ended questioning or silent guessing.\n\n");
    for (id, elicitation) in &spec.elicitations {
        write_elicitation(output, id, elicitation);
    }
}

fn write_elicitation(output: &mut String, id: &str, elicitation: &Elicitation) {
    let _ = writeln!(output, "### `{id}`");
    let _ = writeln!(output, "- question: {}", elicitation.question);
    if let Some(default) = &elicitation.default {
        let _ = writeln!(output, "- default: `{default}`");
    }
    if let Some(max_choices) = elicitation.max_choices {
        let _ = writeln!(output, "- max_choices: {max_choices}");
    }
    if !elicitation.required_when.is_empty() {
        output.push_str("- required_when:\n");
        for condition in &elicitation.required_when {
            if let Some(route) = &condition.route {
                let _ = writeln!(output, "  - route: `{}`", route.0);
            }
            if let Some(missing) = &condition.missing {
                let _ = writeln!(output, "  - missing: `{missing}`");
            }
            if let Some(predicate) = &condition.predicate {
                output.push_str("  - predicate:\n");
                write_predicate(output, predicate);
            }
        }
    }
    output.push_str("- choices:\n");
    for choice in &elicitation.choices {
        write_elicitation_choice(output, choice);
    }
    output.push('\n');
}

fn write_elicitation_choice(output: &mut String, choice: &ElicitationChoice) {
    let _ = writeln!(output, "  - `{}`: {}", choice.id, choice.label);
    if let Some(description) = &choice.description {
        let _ = writeln!(output, "    description: {description}");
    }
    if !choice.sets.is_empty() {
        output.push_str("    sets:\n");
        write_yaml_map(output, &choice.sets, 6);
    }
    if let Some(route) = &choice.route {
        let _ = writeln!(output, "    route: `{}`", route.0);
    }
    if let Some(next) = &choice.next {
        let _ = writeln!(output, "    next: `{next}`");
    }
    if let Some(safety) = &choice.safety {
        let _ = writeln!(output, "    safety: `{}`", safety_name(safety));
    }
}

fn write_trace(output: &mut String, spec: &SkillSpec) {
    let Some(trace) = &spec.trace else {
        return;
    };
    output.push_str("## Decision Trace\n\n");
    output.push_str("When the `skillspec` CLI or a compatible harness evaluates this spec, record the decision path as append-only events. Rules trigger decisions; the evaluator writes the trace.\n\n");
    output.push_str("- mode: `event_log`\n");
    let _ = writeln!(output, "- required: {}", trace.required);
    if !trace.record.is_empty() {
        let events = trace
            .record
            .iter()
            .map(|event| format!("`{}`", trace_event_name(event)))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(output, "- record: {events}");
    } else {
        output.push_str("- record: all v0 decision events\n");
    }
    output.push('\n');
}

fn write_predicate(output: &mut String, predicate: &Predicate) {
    output.push_str("- when:\n");
    if !predicate.user_says_any.is_empty() {
        let _ = writeln!(
            output,
            "  - user_says_any: {}",
            quoted_list(&predicate.user_says_any)
        );
    }
    if let Some(value) = predicate.task_recurrence_likely {
        let _ = writeln!(output, "  - task_recurrence_likely: {value}");
    }
    if let Some(value) = predicate.domain_object_task {
        let _ = writeln!(output, "  - domain_object_task: {value}");
    }
    if let Some(value) = predicate.interactive_prompt_likely {
        let _ = writeln!(output, "  - interactive_prompt_likely: {value}");
    }
    if let Some(value) = predicate.command_likely_long_running {
        let _ = writeln!(output, "  - command_likely_long_running: {value}");
    }
}

fn write_states(output: &mut String, spec: &SkillSpec) {
    if spec.states.is_empty() {
        return;
    }
    output.push_str("## State Machine\n\n");
    output.push_str("Use states as lifecycle guidance. State actions must reference commands or closures; snippets supply user-facing copy.\n\n");
    for (id, state) in &spec.states {
        write_state(output, id, state);
    }
}

fn write_state(output: &mut String, id: &str, state: &State) {
    let _ = writeln!(output, "### `{id}`");
    if !state.r#do.is_empty() {
        let _ = writeln!(output, "- do: {}", code_list(&state.r#do));
    }
    if let Some(say) = &state.say {
        let _ = writeln!(output, "- say: `{say}`");
    }
    if let Some(ask) = &state.ask {
        let _ = writeln!(output, "- ask: `{ask}`");
    }
    if let Some(next) = &state.next {
        let _ = writeln!(output, "- next: `{next}`");
    }
    if let Some(yes) = &state.yes {
        let _ = writeln!(output, "- yes: `{yes}`");
    }
    if let Some(no) = &state.no {
        let _ = writeln!(output, "- no: `{no}`");
    }
    output.push('\n');
}

fn write_commands(output: &mut String, spec: &SkillSpec) {
    if spec.commands.is_empty() {
        return;
    }
    output.push_str("## Command Templates\n\n");
    output.push_str("Command templates are examples and contracts, not automatic permission. Apply the safety class and the harness approval policy before executing.\n\n");
    for (id, command) in &spec.commands {
        write_command(output, id, command);
    }
}

fn write_command(output: &mut String, id: &str, command: &CommandTemplate) {
    let _ = writeln!(output, "### `{id}`");
    if let Some(description) = &command.description {
        let _ = writeln!(output, "- description: {description}");
    }
    if let Some(safety) = &command.safety {
        let _ = writeln!(output, "- safety: `{}`", safety_name(safety));
    }
    output.push_str("- template:\n\n");
    output.push_str("```bash\n");
    output.push_str(&command.template);
    output.push_str("\n```\n");
    if !command.requires.is_empty() {
        output.push_str("- requires:\n");
        write_yaml_map(output, &command.requires, 2);
    }
    if !command.parse.is_empty() {
        output.push_str("- parse:\n");
        for (key, value) in &command.parse {
            let _ = writeln!(output, "  - `{key}`: `{value}`");
        }
    }
    if !command.success_when.is_empty() {
        output.push_str("- success_when:\n");
        write_yaml_map(output, &command.success_when, 2);
    }
    output.push('\n');
}

fn write_snippets(output: &mut String, spec: &SkillSpec) {
    if spec.snippets.is_empty() {
        return;
    }
    output.push_str("## Snippets\n\n");
    for (id, snippet) in &spec.snippets {
        let _ = writeln!(output, "### `{id}`");
        output.push_str(snippet.text.trim());
        output.push_str("\n\n");
    }
}

fn write_closures(output: &mut String, spec: &SkillSpec) {
    if spec.closures.is_empty() {
        return;
    }
    output.push_str("## Closures\n\n");
    output.push_str("Closures are post-task obligations or named lifecycle actions. Run them when referenced by states or `after_success`.\n\n");
    for (id, closure) in &spec.closures {
        let _ = writeln!(output, "### `{id}`");
        write_yaml_block(output, closure);
        output.push('\n');
    }
}

fn write_tests(output: &mut String, spec: &SkillSpec) {
    if spec.tests.is_empty() {
        return;
    }
    output.push_str("## Scenario Tests\n\n");
    output.push_str("Use these as behavioral examples. The agent should make the same routing and guardrail choices for equivalent tasks.\n\n");
    for test in &spec.tests {
        write_test(output, test);
    }
}

fn write_test(output: &mut String, test: &ScenarioTest) {
    let _ = writeln!(output, "### {}", test.name);
    let _ = writeln!(output, "- input: {:?}", test.input);
    if let Some(route) = &test.expect.route {
        let _ = writeln!(output, "- expect route: `{}`", route.0);
    }
    if !test.expect.route_order.is_empty() {
        let _ = writeln!(
            output,
            "- expect route_order: {}",
            test.expect
                .route_order
                .iter()
                .map(|route| format!("`{}`", route.0))
                .collect::<Vec<_>>()
                .join(" -> ")
        );
    }
    if !test.expect.forbid.is_empty() {
        let _ = writeln!(
            output,
            "- expect forbid: {}",
            code_list(&test.expect.forbid)
        );
    }
    if !test.expect.elicit.is_empty() {
        let _ = writeln!(
            output,
            "- expect elicit: {}",
            code_list(&test.expect.elicit)
        );
    }
    if !test.expect.after_success.is_empty() {
        let _ = writeln!(
            output,
            "- expect after_success: {}",
            code_list(&test.expect.after_success)
        );
    }
    output.push('\n');
}

fn write_proof(output: &mut String, spec: &SkillSpec) {
    let Some(proof) = &spec.proof else {
        return;
    };
    if proof.metrics.is_empty() {
        return;
    }
    output.push_str("## Proof Metrics\n\n");
    for metric in &proof.metrics {
        let _ = writeln!(output, "- `{metric}`");
    }
    output.push('\n');
}

fn write_review_required(output: &mut String, spec: &SkillSpec) {
    if spec.review_required.is_empty() {
        return;
    }
    output.push_str("## Review Required\n\n");
    for note in &spec.review_required {
        let _ = writeln!(output, "- {note}");
    }
    output.push('\n');
}

fn write_runtime_commands(output: &mut String) {
    output.push_str("## SkillSpec CLI Commands\n\n");
    output.push_str("Use these commands when the `skillspec` CLI is available. Replace `<skill-folder>` with the folder containing this generated `SKILL.md`. The default trace location is `${PWD}/.skillspec/traces`, where `PWD` is the task working directory.\n\n");
    output.push_str("```bash\n");
    output.push_str("skillspec validate <skill-folder>/skill.spec.yml\n");
    output.push_str("skillspec test <skill-folder>/skill.spec.yml\n");
    output.push_str(
        "skillspec decide <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n",
    );
    output.push_str("skillspec explain <skill-folder>/skill.spec.yml --input='<user task>' --trace-dir \"${PWD}/.skillspec/traces\"\n");
    output.push_str("skillspec trace compact \"${PWD}/.skillspec/traces/<run-id>\"\n");
    output.push_str("```\n");
}

fn write_yaml_block(output: &mut String, value: &serde_yaml::Value) {
    output.push_str("```yaml\n");
    for line in yaml_lines(value) {
        let _ = writeln!(output, "{line}");
    }
    output.push_str("```\n");
}

fn write_yaml_map(output: &mut String, map: &BTreeMap<String, serde_yaml::Value>, indent: usize) {
    let prefix = " ".repeat(indent);
    for (key, value) in map {
        match scalar_yaml(value) {
            Some(value) => {
                let _ = writeln!(output, "{prefix}- `{key}`: {value}");
            }
            None => {
                let _ = writeln!(output, "{prefix}- `{key}`:");
                for line in yaml_lines(value) {
                    let _ = writeln!(output, "{prefix}  {line}");
                }
            }
        }
    }
}

fn yaml_lines(value: &serde_yaml::Value) -> Vec<String> {
    let rendered = serde_yaml::to_string(value).unwrap_or_else(|_| format!("{value:?}"));
    rendered
        .lines()
        .filter(|line| !line.trim().is_empty() && *line != "---")
        .map(|line| line.trim_end().to_owned())
        .collect()
}

fn scalar_yaml(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Bool(value) => Some(value.to_string()),
        serde_yaml::Value::Number(value) => Some(value.to_string()),
        serde_yaml::Value::String(value) => Some(format!("{value:?}")),
        serde_yaml::Value::Null => Some("null".to_owned()),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => None,
    }
}

fn code_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn quoted_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn safety_name(safety: &SafetyClass) -> &'static str {
    match safety {
        SafetyClass::ReadOnly => "read_only",
        SafetyClass::LocalRead => "local_read",
        SafetyClass::LocalWrite => "local_write",
        SafetyClass::NetworkRead => "network_read",
        SafetyClass::NetworkWrite => "network_write",
        SafetyClass::BrowserAttach => "browser_attach",
        SafetyClass::CredentialRequest => "credential_request",
        SafetyClass::Destructive => "destructive",
    }
}

fn trace_event_name(event: &TraceEventKind) -> &'static str {
    match event {
        TraceEventKind::InputReceived => "input_received",
        TraceEventKind::SpecLoaded => "spec_loaded",
        TraceEventKind::RuleEvaluated => "rule_evaluated",
        TraceEventKind::RuleMatched => "rule_matched",
        TraceEventKind::RouteSelected => "route_selected",
        TraceEventKind::RouteOrderSet => "route_order_set",
        TraceEventKind::ForbidAdded => "forbid_added",
        TraceEventKind::AllowAdded => "allow_added",
        TraceEventKind::ElicitationRequested => "elicitation_requested",
        TraceEventKind::AfterSuccessScheduled => "after_success_scheduled",
        TraceEventKind::OutcomeRecorded => "outcome_recorded",
    }
}

fn skill_name(id: &str) -> String {
    let mut name = String::new();
    let mut last_was_dash = false;
    for char in id.chars() {
        let next = if char.is_ascii_alphanumeric() {
            last_was_dash = false;
            Some(char.to_ascii_lowercase())
        } else if !last_was_dash {
            last_was_dash = true;
            Some('-')
        } else {
            None
        };
        if let Some(char) = next {
            name.push(char);
        }
    }
    name.trim_matches('-').to_owned()
}
