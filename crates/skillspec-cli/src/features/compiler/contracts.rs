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
    output.push_str("- Quote YAML string values that contain `: `, especially `elicitations.*.question`, descriptions, `steps[].note`, recipe/procedure notes, and review notes.
");
    output.push_str("- Artifact `produced_by` and `consumed_by` entries can only reference `command`, `code`, or `recipe`; use route checks, recipe steps, or imports/resources `used_by` for route-level linkage.
");
    output.push_str("- For one atomic prose skill, prefer `skillspec port-one-shot <source> --out <draft> --target codex-skill --prove` before hand-editing. It writes grammar/schema proof, a typed shape crib, source map, doctor report, mechanical draft, QA results, compile output, and optional estimated summary stats when `--run-dir` is supplied.

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
    output.push_str("- For each execution phase, run `skillspec act <skill-folder>/skill.spec.yml --input='<task>' --run <run_dir> --phase <phase-id>` before acting, obey the rendered phase tool boundary, record phase progress in `<run_dir>/execution.jsonl`, then use `skillspec progress show <skill-folder>/skill.spec.yml --run <run_dir>` as an internal gate check for completed, current, blocked, and remaining phases. Surface only the gate result unless the user asks for details or a blocker/failure needs evidence.\n");
    output.push_str("- If `skillspec plan`, `skillspec act`, or `skillspec progress` is unavailable, fall back to `skillspec decide` plus a manually constructed allow/deny checklist and progress notes before tool use. Prefer `skillspec explain` for human-facing route rationale.\n");
    output.push_str("- After `skillspec act`, inspect matched rules and active execution surfaces with `skillspec query <skill-folder>/skill.spec.yml <handle> --view summary` and `skillspec refs <skill-folder>/skill.spec.yml <handle> --view summary` instead of ad hoc YAML queries.\n");
    output.push_str("- Escalate query detail from `--view index` to `--view summary` to `--view full` only when the smaller view cannot answer the decision.\n");
    output.push_str("- Token economy applies to direct SkillSpec CLI runs: prefer compact CLI summaries, source-map handles, query/ref summaries, alignment rows, and artifact paths; load full reports or full source spans only when exact evidence is required.\n");
    output.push_str("- When invoking `skillspec plan`, `skillspec act`, or `skillspec decide`, pass only the user's task text. Strip skill invocation prefixes such as `/durable-executor-spec`, `$durable-executor-spec`, or `/my-skill` before setting `--input`.\n");
    output.push_str("- Prefer `--input='<task text>'` in shell examples so `$skill-name` text is not expanded by the shell.\n");
    output.push_str("- Resolve `skill.spec.yml` relative to this `SKILL.md` folder, not the process working directory.\n");
    output.push_str("- Always pass `--trace-dir`; use `${PWD}/.skillspec/traces` unless the user or harness provides a run-specific trace directory.\n");
    output.push_str("- After `skillspec plan` or `skillspec act` prints trace lines, keep the emitted `run_dir` and mention it when reporting how the decision was made.\n");
    output.push_str("- When the CLI is available, run `skillspec trace align <skill-folder>/skill.spec.yml --decision-trace <run_dir> --summary --proof-digest <run_dir>/proof-digest.json` and add `--execution-trace <run_dir>/execution.jsonl` when structured action evidence exists. This writes `<run_dir>/alignment.json` and a grouped proof digest. Include only the compact alignment summary, token block, digest path, and trace path unless debugging, failure, or user request requires detailed checks. Do not report a bare `unproven`; use `Alignment: partial` plus specific `Missing proof` rows.\n");
    output.push_str("- Always include token usage in the completion report. For durable-executor runs, collect durable workspace stats into a report file and run `skillspec progress stats <run_dir> --workspace <workspace> --workspace-stats-report <file> --phase <phase-id> --requirement <stats-requirement-id>` before alignment; missing `stats_collected` evidence is a workflow bug, not a normal omission. Draft the final response with Result, Evidence, Alignment summary, Token usage, and SkillSpec sections, run one initial `skillspec trace align --summary --proof-digest <run_dir>/proof-digest.json`, batch any real missing proof rows into `<run_dir>/final-proof.jsonl`, run `skillspec progress batch <run_dir> --events <run_dir>/final-proof.jsonl` once if there are multiple rows, run `skillspec progress final-response <run_dir> --phase <phase-id> --requirement <report-requirement-id> --result --evidence --alignment --token-savings`, then rerun `skillspec trace align --summary` once and report that compact final alignment. Do not rerun alignment after each individual progress row. Use measured `Token consumption` and `Token savings` from `skillspec trace align --summary` when available; if stats truly cannot be collected, say `not recorded`. For direct CLI runs, record `--summary` metrics first with `skillspec progress stats <run_dir> --agent-visible-tokens <n> --artifact-tokens-preserved <n> --avoided-tokens <n> --metrics-source estimated`, then report them as estimated output economy, not measured model usage. In direct runs, the Evidence section should name trace files and artifacts only; do not mention durable-executor or its underlying tools unless that route was actually used. Do not invent replay savings.\n");
    output.push_str("- Alignment proof rows may be batched through `<run_dir>/final-proof.jsonl`; use the grouped proof digest as a checklist and record real evidence once with `skillspec progress batch`.\n");
    output.push_str("- Alignment proof rows may mention command basenames such as `gh` or `git`, but must not include raw command arguments because args may contain private data.\n\n");
    output.push_str("Minimum final response shape:\n\n");
    output.push_str("- `Result`: answer the user's task directly.\n");
    output.push_str("- `Evidence`: for durable-executor runs, workspace name plus important response ids/files the user can query later; for direct CLI runs, trace and artifact paths only.\n");
    output.push_str("- `Alignment summary`: include `Decision replay`, `Phase order`, `Requirements`, one or more `Missing proof` rows, `Forbidden actions`, and `Alignment` exactly as reported by `skillspec trace align --summary`.\n");
    output.push_str("- `Token usage`: include measured `Token consumption` and `Token savings` from `skillspec trace align --summary` when available; otherwise include estimated summary metrics or say `not recorded`.\n");
    output.push_str("- `SkillSpec`: selected route, trace run directory, align status, status meaning, and proof rows that map request/spec obligations to observed evidence. Never let this replace the Result, Evidence, Alignment summary, or Token usage sections.\n\n");
}

pub(super) fn write_harness_presentation_contract(output: &mut String) {
    output.push_str("## Harness Presentation Contract\n\n");
    output.push_str("- When presenting plan, action, progress, command, recipe, or closure steps to a user, show the step `description` as the default visible text. If no description is present, show a humanized id.\n");
    output.push_str("- Keep raw command templates, concrete argv, provider payloads, and low-level tool details collapsed by default in normal progress UI. Reveal them only when the user explicitly expands details, approval is required, a command fails, debug/verbose mode is active, or no usable description exists.\n");
    output.push_str("- For approval prompts, destructive or externally mutating actions, and failure reports, show both the human description and the raw command or payload summary needed for informed approval/debugging.\n");
    output.push_str("- This is presentation-only. Always preserve raw command templates, concrete executed commands, stdout/stderr handles, response ids, and files in trace/evidence/alignment data exactly as required by the active SkillSpec.\n\n");
}
