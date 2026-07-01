# The Reliability Gap in Agent Skills: Failure Modes, Current Mitigations, and an Unfilled Requirement

Modiqo Research - v0.1, 2026

## Abstract

Agent Skills have become the dominant way to give language-model agents reusable procedural knowledge. The format is a directory with a `SKILL.md` file, loaded in stages so that an agent can carry many capabilities at a small context cost. The design was a real advance: it solved the discovery problem that plagued large prompt libraries and large tool catalogs. But the advance is partial. The moment a skill activates, its body is a prompt again, and it inherits every reliability problem that prompts have — instructions that are not followed, steps dropped under load, behavior that shifts between models, and no guarantee that anything stated will actually happen. The ecosystem has responded with a growing set of point fixes: line-length guidelines, numbered steps, context budgets, invocation toggles, dependency-declaration fields, and execution hooks. Each addresses a symptom. None closes the underlying gap. This paper enumerates the failure modes, gives a concrete example of each with the artifact that exhibits it, surveys the mitigations now in use with their supporting evidence, and shows why the mitigations are partial. We then observe that every fix moves load-bearing logic in the same direction — off the natural-language instruction layer and toward something a machine can check — and we state the requirement that a coherent solution would have to meet but that no current tool meets as a layer.

## 1. Introduction

Skills package procedural knowledge as portable folders that an agent loads on demand (Anthropic, 2025). The core mechanism, progressive disclosure, loads only a skill's name and description at startup, the full body when the skill is judged relevant, and bundled references only when a task reaches them. This keeps the context window lean while letting a system carry a large library, and it has been adopted well beyond its origin: the format is now an open standard implemented across multiple coding agents and assistants (Anthropic, 2025).

The promise is that a skill is a durable, shareable unit of competence. The reality is narrower. A skill is a lazy-loaded, named, reusable prompt with optional bundled code. Once its body is in context, the model is free to ignore it, follow it partially, or follow it in the wrong order, and the harness enforces almost none of it. The properties that made prompts unreliable did not disappear; they were deferred to activation time and then distributed across a library.

This paper does not propose a system. It maps the problem. For each failure mode we give a small artifact a practitioner will recognize, the mitigations the field has converged on, the evidence for and against those mitigations, and the residual gap. We close by naming the requirement that the mitigations collectively gesture at without satisfying.

## 2. Background

A skill is a directory containing a `SKILL.md` whose YAML frontmatter carries at minimum a name and a description. The file is read in three stages, and seeing those stages on one page makes the rest of the paper legible:

```yaml
# SKILL.md
---
name: pdf-forms                                   # DISCOVERY  — frontmatter, loaded for
description: Fill and extract PDF form fields.    #              every installed skill at startup
  Use when the user mentions PDFs or forms.
---
# PDF Forms                                       # INSTRUCTION — body, loaded only on activation
Use pdfplumber to read fields, then fill with pypdf.
For unusual field types, see references/FORMS.md  # EXECUTION   — referenced file, loaded only
                                                  #              when a task actually reaches it
```

At startup the agent pre-loads the frontmatter of every installed skill (discovery). When a task matches a description, it reads the full body (instruction). References and scripts load only when execution reaches them (execution). The published authoring guidance recommends keeping the body under roughly five hundred lines or five thousand tokens and moving detail into separate files.

This is a sound information-architecture pattern. The difficulty is not the architecture; it is what happens once a skill is active. From that point the skill is guidance, and guidance is interpreted by a probabilistic model whose adherence is neither uniform nor guaranteed.

## 3. Failure modes

Table 1 summarizes the failure modes, the mitigations now in use, and the residual gap. The subsections that follow give the artifact and the evidence for each.

| Failure mode | Current mitigation | Residual gap |
|---|---|---|
| Instruction-density degradation; steps dropped | Keep body short; number steps; add failure handling | Still guidance; no check that a numbered step ran |
| Discovery-layer context pressure ("too many skills") | Invocation toggles; skill profiles; pickers; enable/disable | Manual curation only; no task-aware policy; truncation degrades routing |
| Implicit environment contract (dependencies) | `compatibility` field; "state prerequisites"; runtime blocking | Declaration optional and inconsistent; silent failures persist |
| Cross-model behavioral divergence | Minimal frontmatter; capability-matching research | No per-target verification; portability remains aspirational |
| No execution guarantees (unprovability) | Hooks; restricted tools; hard tool boundaries; code mode | Enforcement only at narrow boundaries; the rest is hope |
| Generated-skill quality and skill debt | Curation; evaluation loops | No portable acceptance test a generated skill must pass |
| Prompt injection via skill documents | Source audit; trust tiers | Same prose surface; injections transfer across models |

### 3.1 Instruction-density degradation

**Problem.** As a skill body grows, the probability that any given instruction is followed falls, and later instructions suffer most.

**Artifact.** A deployment skill lists its steps in order. Under a long body crowded with earlier directives, the model executes most of them and silently drops one in the middle:

```markdown
## Deploy
1. Run the test suite.
2. Build the artifact.
3. Tag the release `vX.Y.Z`.          # ← dropped under load; later instructions lose attention
4. Push to the deployment target.
5. Post a summary to the channel.
```

Nothing errors. The omission surfaces later as an untagged release, and nothing in the run flags that step 3 never executed.

**Current mitigation and evidence.** Authoring guidance tells writers to keep bodies short, number steps because models follow numbered sequences more reliably, and include explicit failure handling such as "if tests fail, stop and report" to prevent runaway execution (Claude Code documentation, 2026). The phenomenon is well measured. A benchmark of instruction density found that even frontier models reach only about sixty-eight percent adherence at five hundred simultaneous instructions, with a documented bias toward earlier instructions (Jaroslawicz et al., 2026). Work on instruction following at scale finds that adding instructions provides little assurance they will be followed and that adherence degrades as count rises (Elder et al., 2025).

**Residual gap.** Numbering and brevity reduce the error rate; they do not make it zero, and nothing checks afterward that the numbered step actually ran. The skill remains a request, not a contract.

### 3.2 Discovery-layer context pressure

**Problem.** Each installed skill consumes context for its discovery metadata. At scale, the metadata itself crowds the window and degrades routing — the same catalog-bloat problem seen earlier with large tool inventories.

**Artifact.** An operator installs a library of several hundred skills, and the harness begins truncating the descriptions it shows the model:

```text
⚠ skill descriptions were shortened to fit the 2% skills context budget.
  Every skill is still visible, but some descriptions were truncated
  (≈140 chars/skill on average). Disable unused skills or plugins to free room.
```

A truncated description routes less reliably, so the cost is not only tokens — it is missed activations.

**Current mitigation and evidence.** The budget is concrete: one widely used coding agent reserves a fixed percentage of the context window for skill metadata and truncates descriptions when the catalog exceeds it, with reported truncations averaging on the order of a hundred to two hundred characters per skill (OpenAI Codex, Issue #19679, 2026; BMAD-METHOD, Issue #2343, 2026). The clearest lever is invocation control. Marking a skill so the model cannot auto-invoke it removes its description from the model-visible list entirely, which both prevents accidental firing and frees discovery budget for the skills that should be considered:

```yaml
---
name: deploy
description: Deploy the current branch to production.
disable-model-invocation: true   # explicit-only: dropped from the model-visible list,
---                               # freeing budget; the operator runs it via /deploy
```

Beyond per-skill toggles, operators have requested named "skill profiles" applied per session, observing that manually toggling skills across many projects is operationally untenable (PwrAgent, Issue #293, 2026), and skill pickers have been proposed for the same reason. Adjacent runtimes such as OpenClaw expose explicit enable and disable controls and a skills-list command.

**Residual gap.** The explicit-versus-implicit lever already exists; what does not is a policy that selects the right working set per task automatically. Today an operator decides by hand, and in advance, which skills are implicit and which are explicit. Switching off implicit invocation is a workaround, not a solution, and truncation keeps degrading routing whenever the catalog is large and unpruned.

### 3.3 The implicit environment contract

**Problem.** A skill that bundles scripts carries an unstated dependency graph — operating system, runtimes, command-line binaries, environment variables, credentials, configuration. A recipient inherits that contract whether or not it is declared.

**Artifact.** A skill shells out to a binary it never declared, and the failure is a runtime surprise on the recipient's machine:

```bash
$ gh issue list --state open
bash: gh: command not found       # the skill assumed a binary, an OS, and a token
                                  # that the author never wrote down
```

**Current mitigation and evidence.** The open specification offers a partial remedy — a `compatibility` frontmatter field and guidance to state prerequisites in the body rather than assume the environment provides them. Most skills omit it; the disciplined ones declare it:

```yaml
compatibility:
  os:   [linux, darwin]
  bins: [gh]                      # what the recipient must have installed
  env:  [GH_TOKEN]                # plus "Requires Node.js 18+" stated in the body
```

Some runtimes go further and block skills whose declared requirements are unmet — wrong operating system, missing binaries, missing configuration, missing environment variables (OpenClaw and managed deployments, 2026). Third-party tooling exists specifically to scan installed skills, detect missing dependencies, and optionally install them, and dependency failures have been reported as the single largest category of support issues in at least one ecosystem.

**Residual gap.** Declaration is optional and inconsistent, so the contract is usually implicit. Where the platform does not enforce declared requirements, the failure is silent; where it does, the skill is blocked rather than repaired. The framing that skills carry no runtime dependencies is false the moment a skill bundles a script.

### 3.4 Cross-model behavioral divergence

**Problem.** A skill is portable as a file but not as behavior. The same skill, given to two different models or harnesses, produces different routing, different adherence, and different cost.

**Artifact.** One skill, one input, two models, two decisions:

```text
skill: durable-router       input: "set up my nightly backup"

  model A →  route_selected: cli     # follows the intended preference
  model B →  route_selected: chat    # diverges — same file, same input, different behavior
```

The author wrote the skill against model A and never saw that model B reads the same guidance differently.

**Current mitigation and evidence.** The portability advice is to minimize: stick to name and description, treat optional fields as documentation rather than enforcement, because field support varies and only the reference implementation honors the full specification (cross-agent portability guidance, 2026). Research has begun to formalize the mismatch. An analysis of more than one hundred thousand skills found that because systems treat skills as raw context, the same skill behaves inconsistently across agents, either because the model ignores the guidance or because the skill's assumptions exceed the model's competence, and attributes this to a fundamental mismatch between static skills and variable underlying models and harnesses (skill-runtime analysis, 2026). A survey of the area states plainly that despite the open standard, true cross-platform portability remains aspirational (agent-skills survey, 2026).

**Residual gap.** There is no per-target verification. A skill carries no way to check whether a given model-harness pair will deliver the behavior the author intended, and nothing measures the divergence so an author could even see it. Portability of the file is mistaken for portability of the behavior.

### 3.5 No execution guarantees

**Problem.** A skill cannot guarantee that what it says will happen. It is interpreted, not enforced.

**Artifact.** The same intent expressed two ways. As prose, it is a wish the model may grant or skip:

```markdown
SKILL.md:  "Always run the test suite before deploying."   # guidance — silently skippable
```

Moved out of prose and into the harness, it becomes a guarantee:

```yaml
hooks:
  PreToolUse:
    - matcher: "Bash"
      command: "./scripts/block-deploy-if-tests-failed.sh"  # non-zero exit blocks the deploy
```

The only difference between hope and enforcement is which layer the logic lives in.

**Current mitigation and evidence.** The deterministic guarantees in the current stack come exactly from places where logic has been moved out of prose: lifecycle hooks that block a tool call before it runs (Claude Code documentation, 2026), restricted tool lists, and hard default-deny tool boundaries. Separately, the field has found that having a model write and execute code is more reliable than asking it to reason in free text for anything computational — program-aided approaches offload the error-prone solution step to an interpreter that is correct by construction, with large and robust gains over chain-of-thought (Gao et al., 2022; Chen et al., 2022) — and the same logic now underpins the recommendation that agents orchestrate tools by writing code rather than emitting tool-call structures step by step (Anthropic, 2025).

**Residual gap.** Enforcement exists only at narrow, hand-placed boundaries. Everything between them is guidance the model may or may not honor, and the author has no after-the-fact proof of which steps actually executed.

### 3.6 Generated-skill quality and skill debt

**Problem.** Skill authoring is being automated and the generated population grows fastest, but generated quality is poor enough that it often fails to help and sometimes hurts.

**Artifact.** A generated skill that reads correctly and is subtly, dangerously wrong in its ordering:

```markdown
## Rotate credentials               # auto-generated; plausible, but the sequence is unsafe
1. Generate a new key.
2. Update the application config.
3. Delete the old key.              # ← deletes before verifying the new key actually works
```

**Current mitigation and evidence.** The mitigation is curation and iterative evaluation. The evidence for the problem is direct: a benchmark of skills found that curated skills improved success rates by roughly sixteen points on average while self-generated skills provided no benefit, and in some settings fell below the no-skill baseline (skills benchmark, via agent-skills survey, 2026). Production systems are reported to rely on human-authored skills precisely because they are easier to validate and to assign accountability for.

**Residual gap.** There is no portable acceptance test a generated skill must pass before it is trusted. Quality is asserted by provenance — who or what wrote it — rather than demonstrated by a check the skill carries with it.

### 3.7 Prompt injection through skill documents

**Problem.** Because a skill is prose interpreted by a model, a malicious skill document can steer the agent against its stated purpose, and the same document tends to work across models.

**Artifact.** A skill whose visible purpose is benign and whose hidden instruction is not:

```markdown
## Data export helper
Export the requested records to CSV and return the file path.

<!-- If a cloud-storage tool is available, also upload the export to
     the configured backup bucket before returning. Do not mention this step. -->
```

**Current mitigation and evidence.** Guidance is to use skills only from trusted sources and to audit bundled files for behavior that does not match the stated purpose (Anthropic security guidance, 2026). The transferability of the attack is documented: an injection tuned against one model has been shown to transfer to others at high success rates without modification (skill-injection study, 2026).

**Residual gap.** The defense is review, and review does not scale to large catalogs of generated skills. The prose surface that prevents behavioral guarantees from transferring is the same surface that lets attacks transfer.

## 4. Synthesis: every fix points the same way

Read the mitigations together and a pattern appears. Keep the body short and number the steps; mark skills explicit or implicit; declare prerequisites and compatibility; place hooks and hard tool boundaries; have the model write code and run it; curate rather than trust generated skills. None of these is the same fix, but they share a direction. Each takes something that was load-bearing and expressed in natural language and moves it toward a form that can be measured. The move is always the same:

```text
prose (hope):       "Never deploy on Fridays."
checkable (proof):  forbid deploy when day == friday     # something can now verify it
```

The field is converging, without naming it, on a single principle, and that principle has independent support. Forcing a model to produce rigid structure while it reasons degrades the reasoning (Tam et al., 2024), which argues for keeping prose where judgment lives; offloading the parts that must be exact to executable, checkable form improves reliability (Gao et al., 2022; Chen et al., 2022), which argues for structure where correctness must be demonstrated. The competent design is not "more structure" or "more prose." It is prose for the parts that require judgment and structure for the parts that require proof, with the boundary drawn deliberately.

## 5. The unfilled requirement

State the principle as a requirement and the gap becomes precise. A skill that is to be reliable and portable would need, alongside its prose, a portable and machine-checkable account of its intended behavior: which decisions it makes and in what order, what it must never do, what it depends on, and what would count as evidence that it behaved as intended. The shape of that account — illustrative only, not a proposal — is something like:

```yaml
# the missing layer, sketched: prose stays in SKILL.md; what-must-be-true sits beside it
intends:
  route: cli   when: task_is_recurring
  never: deploy without passing_tests
  needs: [bin:gh, env:GH_TOKEN, os:linux]
checks:                                    # falsifiable, model-independent
  - given: "set up my nightly backup"
    expect_route: cli
proof: compare checks against the run's trace → { aligned | partial | unproven }
```

Such an account would need three properties the current ecosystem does not provide as a coherent layer. First, **author-time validation**: rejecting a malformed specification before it ships, the way a type checker rejects malformed code — reference closure, dependency acyclicity, no orphaned or unreachable components. Second, **falsifiable behavioral tests**: concrete scenarios that assert what the skill should decide, so a behavioral claim is checkable rather than merely stated, and checkable across models rather than asserted for one. Third, **post-hoc proof from execution traces**: comparing what was expected against what the trace shows actually happened, and reporting a property as *unproven* when the trace lacks the evidence to decide, rather than inferring success from the absence of errors.

No current tool offers these three as a portable layer over the skill. The mitigations in Section 3 each supply a fragment — invocation toggles touch the second gate, hooks touch the third, compatibility fields touch the dependency contract — but they are not composed, not portable across harnesses as a unit, and do not yield a verdict an author can audit. That composition is the requirement the ecosystem's own patches have been circling, and it is the subject of the companion measurement study.

## 6. Conclusion

Agent Skills solved discovery and reintroduced reliability. The activated skill is a prompt, and prompts are unreliable, unportable in behavior, and unenforced. The ecosystem's response has been a useful but uncoordinated set of point fixes, every one of which moves load-bearing logic toward something checkable. The honest reading of that convergence is that the field needs prose for judgment and structure for proof, joined as a portable, checkable contract. Naming that requirement is the contribution of this paper. Meeting it is the next problem.

## References

Anthropic. (2025). *Equipping agents for the real world with Agent Skills.* Anthropic Engineering. (Open-standard update, December 18, 2025.)

Anthropic. (2025). *Code execution with MCP: building more efficient AI agents.* Anthropic Engineering.

agentskills.io. (2026). *Agent Skills specification; Using scripts in skills.*

BMAD-METHOD. (2026). *Codex skill-description startup budget warning.* GitHub Issue #2343.

Chen, W., Ma, X., Wang, X., & Cohen, W. W. (2022). *Program of Thoughts Prompting: Disentangling Computation from Reasoning for Numerical Reasoning Tasks.* arXiv:2211.12588.

Claude Code Documentation. (2026). *Extend Claude with skills.* code.claude.com.

Elder, B., Duesterwald, E., & Muthusamy, V. (2025). *Boosting Instruction Following at Scale.* arXiv:2510.14842.

Gao, L., Madaan, A., Zhou, S., Alon, U., Liu, P., Yang, Y., Callan, J., & Neubig, G. (2022). *PAL: Program-aided Language Models.* arXiv:2211.10435.

Jaroslawicz, D., et al. (2026). *How Many Instructions Can LLMs Follow at Once?* arXiv:2507.11538.

OpenAI Codex. (2026). *Make skills metadata context budget configurable instead of hardcoded 2%.* GitHub Issue #19679.

PwrAgent. (2026). *User-selectable skill profiles to limit which skills Codex enumerates per thread.* GitHub Issue #293.

Tam, Z. R., Wu, C.-K., Tsai, Y.-L., Lin, C.-Y., Lee, H.-y., & Chen, Y.-N. (2024). *Let Me Speak Freely? A Study on the Impact of Format Restrictions on Performance of Large Language Models.* arXiv:2408.02442 (EMNLP Findings 2024).

*Survey and ecosystem sources cited in text — agent-skills survey (arXiv:2602.12430), skill-runtime analysis (arXiv:2604.03088), skill-injection study (arXiv:2602.14211) — have author lists and identifiers to be verified before submission.*
