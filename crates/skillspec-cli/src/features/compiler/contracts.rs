pub(super) fn write_durable_handoff_contract(output: &mut String) {
    output.push_str("## Durable Handoff Contract\n\n");
    output.push_str("This skill participates in agent-mediated durable execution. There is no runtime handoff engine: the agent reads the active SkillSpec contracts, carries the handoff packet in context, and preserves the declared evidence.\n\n");
    output.push_str("- If a durable handoff packet is present, preserve its `workspace`, `trace_dir`, `return_to`, `branch_id`, and `execution_policy` fields.\n");
    output.push_str("- If no durable handoff packet is present and the task asks for remembered evidence, future recall, reuse, trace, alignment, or durable execution, route through `durable-executor` first unless the user explicitly requests direct/no-rote execution.\n");
    output.push_str("- If `durable_context.active` is true, do not route the whole task back to `durable-executor`; use `durable-executor` only as the execution substrate and then return to `return_to`.\n");
    output.push_str("- This skill owns its domain interpretation and validation. `durable-executor` owns workspace, trace, evidence, command substrate, final alignment, token-savings, and recall/crystallization closure when it initiated the handoff.\n");
    output.push_str("- Any CLI, shell command, local process, package command, API fallback, or provider command must use the durable execution substrate, normally a rote adapter or `rote exec --`, unless the active spec or user explicitly allows direct execution.\n");
    output.push_str("- On completion, emit a return packet with status, selected route, skill metadata, artifacts, evidence handles, blockers, and trace paths, then hand back to `return_to` for final closure.\n");
    output.push_str("- For parallel work, keep one top-level workspace but use branch-scoped `branch_id`, trace paths, evidence labels, and artifact directories.\n\n");
}

pub(super) fn write_authoring_contract(output: &mut String) {
    output.push_str(
        "## Authoring And Revision Contract

",
    );
    output.push_str(
        "When importing, creating, revising, or extending this SkillSpec-backed skill, use the embedded grammar teacher before editing `skill.spec.yml`:

",
    );
    output.push_str(
        "```bash
",
    );
    output.push_str(
        "skillspec grammar sensemake --view index
",
    );
    output.push_str(
        "skillspec grammar sensemake --view porting
",
    );
    output.push_str(
        "skillspec grammar checklist --for import-skill
",
    );
    output.push_str(
        "```

",
    );
    output.push_str("- Treat the checklist as the review gate for semantic edits: activation, routes, rules, elicitations, imports/resources, commands/deps, procedures, tests, proof, and contract quality.
");
    output.push_str("- Fill or update a coverage matrix with `prose_span | obligation | skillspec_construct | confidence | status | review_note` before installing or releasing a changed skill.
");
    output.push_str("- Use `skillspec grammar schema --json` when a harness needs the exact embedded JSON schema.
");
    output.push_str("- Do not patch YAML by memory when the binary can teach the current grammar. Run the grammar commands again after CLI upgrades or when a spec shape is unfamiliar.

");
}

pub(super) fn write_runtime_contract(output: &mut String) {
    output.push_str("## Runtime Contract\n\n");
    output.push_str("- Read this generated skill for orientation and immediate rules.\n");
    output.push_str("- Treat routes, rules, states, commands, tests, and review notes below as authoritative.\n");
    output.push_str("- Route `execution_plan` entries are ordered hard obligations. Execute phase 1 before phase 2; do not jump to a later handoff just because that substrate is available. Phase `jumps` are the only declared conditional exits from the default order.\n");
    output.push_str("- Route `handoff` entries are hard execution boundaries, not prose. If a selected route has `handoff.boundary: stop_current_skill`, stop current-skill execution except to pass the declared context to the target skill.\n");
    output.push_str("- Rules beat prose when there is tension.\n");
    output.push_str("- `forbid` entries are hard negative steering, not suggestions.\n");
    output.push_str("- `elicit` entries require bounded user questions before guessing.\n");
    output.push_str("- Use the scenario tests as examples of expected behavior.\n");
    output.push_str("- When unfamiliar with the spec shape, run `skillspec sensemake <skill-folder>/skill.spec.yml --view index` to get section roles, counts, ids, and query commands without consuming the whole spec.\n");
    output.push_str("- When the `skillspec` CLI is available, run `skillspec plan <skill-folder>/skill.spec.yml --input='<task>' --trace-dir \"${PWD}/.skillspec/traces\"`, preserve the printed `run_dir`, then run `skillspec act <skill-folder>/skill.spec.yml --input='<task>' --run <run_dir> --phase <phase-id>` before substrate tools. Treat the phase plan and current-route checklist as the active execution SOP.\n");
    output.push_str("- The `skillspec act` checklist is an OODA loop for the selected route: observe the task and trace, orient with matched rules, current phase, and `PHASE TOOL BOUNDARY - HARD`, decide the next allowed action, act with evidence capture, then repeat before the next tool call.\n");
    output.push_str("- The selected route and matched rules override lower-level skill defaults and generic tool preferences. If a lower-level skill suggests a forbidden tool, stop and follow the SkillSpec route.\n");
    output.push_str("- For each execution phase, run `skillspec act <skill-folder>/skill.spec.yml --input='<task>' --run <run_dir> --phase <phase-id>` before acting, obey the rendered phase tool boundary, record phase progress in `<run_dir>/execution.jsonl`, then run `skillspec progress show <skill-folder>/skill.spec.yml --run <run_dir>` to see completed, current, blocked, and remaining phases.\n");
    output.push_str("- If `skillspec plan`, `skillspec act`, or `skillspec progress` is unavailable, fall back to `skillspec decide` plus a manually constructed allow/deny checklist and progress notes before tool use. Prefer `skillspec explain` for human-facing route rationale.\n");
    output.push_str("- After `skillspec act`, inspect matched rules and active execution surfaces with `skillspec query <skill-folder>/skill.spec.yml <handle> --view summary` and `skillspec refs <skill-folder>/skill.spec.yml <handle> --view summary` instead of ad hoc YAML queries.\n");
    output.push_str("- Escalate query detail from `--view index` to `--view summary` to `--view full` only when the smaller view cannot answer the decision.\n");
    output.push_str("- When invoking `skillspec plan`, `skillspec act`, or `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/durable-executor-spec`, `$durable-executor-spec`, or `/my-skill` before setting `--input`.\n");
    output.push_str("- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.\n");
    output.push_str("- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.\n");
    output.push_str("- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.\n");
    output.push_str("- After `skillspec plan` or `skillspec act` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.\n");
    output.push_str("- When the CLI is available, run `skillspec trace align <skill-folder>/skill.spec.yml --decision-trace <run_dir>` and add `--execution-trace <run_dir>/execution.jsonl` when structured action evidence exists. This writes `<run_dir>/alignment.json`. Include the compact alignment summary, status meaning, decision-replay and execution-proof layer results, evidence gaps, user-facing proof rows, and any failed or partial checks in the completion report. Do not report a bare `unproven`; use `Alignment: partial` plus specific `Missing proof` rows.\n");
    output.push_str("- Always include token usage in the completion report. For successful rote-backed runs, collect `rote workspace stats <workspace>` into a report file and run `skillspec progress stats <run_dir> --workspace <workspace> --workspace-stats-report <file> --phase <phase-id> --requirement <stats-requirement-id>` before alignment; missing `stats_collected` evidence is a workflow bug, not a normal omission. Draft the final response with Result, Evidence, Alignment summary, Token usage, and SkillSpec sections, run `skillspec progress final-response <run_dir> --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings`, then rerun `skillspec trace align` and report that final alignment. Use `Token consumption` and `Token savings` from `skillspec trace align`; if stats truly cannot be collected, say `not recorded`. When query-reduction stats exist, report cached response tokens reduced to query-result tokens, saved-token delta, and reduction percentage instead of calling cached tokens consumed prompt tokens. When rote workspace evidence or stats exist, name the workspace and response ids/files, describe the workspace as a retrievable context file system, report measured context-window/API tokens when available, and explain crystallized/remembered reuse as avoiding full evidence reloads. Do not invent replay savings.\n");
    output.push_str("- Alignment proof rows may mention command basenames such as `gh` or `git`, but must not include raw command arguments because args may contain private data.\n\n");
    output.push_str("Minimum final response shape:\n\n");
    output.push_str("- `Result`: answer the user's task directly.\n");
    output.push_str("- `Evidence`: workspace name plus important response ids/files the user can query later.\n");
    output.push_str("- `Alignment summary`: include `Decision replay`, `Phase order`, `Requirements`, one or more `Missing proof` rows, `Forbidden actions`, and `Alignment` exactly as reported by `skillspec trace align`.\n");
    output.push_str("- `Token usage`: include `Token consumption` and `Token savings` exactly as reported by `skillspec trace align`; say `not recorded` when absent.\n");
    output.push_str("- `SkillSpec`: selected route, trace run directory, align status, status meaning, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, Alignment summary, or Token usage sections.\n\n");
}

pub(super) fn write_harness_presentation_contract(output: &mut String) {
    output.push_str("## Harness Presentation Contract\n\n");
    output.push_str("- When presenting plan, action, progress, command, recipe, or closure steps to a user, show the step `description` as the default visible text. If no description is present, show a humanized id.\n");
    output.push_str("- Keep raw command templates, concrete argv, provider payloads, and low-level tool details collapsed by default in normal progress UI. Reveal them only when the user explicitly expands details, approval is required, a command fails, debug/verbose mode is active, or no usable description exists.\n");
    output.push_str("- For approval prompts, destructive or externally mutating actions, and failure reports, show both the human description and the raw command or payload summary needed for informed approval/debugging.\n");
    output.push_str("- This is presentation-only. Always preserve raw command templates, concrete executed commands, stdout/stderr handles, response ids, and files in trace/evidence/alignment data exactly as required by the active SkillSpec.\n\n");
}
