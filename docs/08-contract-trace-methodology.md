# Measuring Behavioral Divergence in Portable Agent Skills: A Contract-and-Trace Methodology

*Preprint — draft v0.1. Empirical tables are scaffolded and to be populated as the corpus grows.*

## Abstract

Agent Skills are portable as files but not as behavior. A skill — a `SKILL.md` document with optional bundled code, loaded into a model's context on demand — is interpreted by a probabilistic model and enforced by almost nothing, so the same skill can route differently, drop steps, and incur different cost across models and harnesses. This divergence is largely invisible to authors because there is no ground truth for what a skill *did* short of its execution trace, and no portable specification of what it was *supposed* to do. We make three contributions. First, we formalize skill behavior as the interaction of three independent non-deterministic gates — activation, adherence, and enforcement — and define realized behavior in terms of execution traces. Second, we propose a measurement methodology built from two artifacts: a *behavioral contract*, a declarative and falsifiable specification of a skill's intended steering and dependencies, evaluated as a deterministic reduction over an ordered rule set; and *trace alignment*, a comparison of contract expectations against a recorded trace that yields a three-valued verdict — *aligned*, *partial*, or *unproven* — where *unproven* is reported when the trace lacks the evidence to decide, rather than inferring success from the absence of errors. Third, we define an experimental protocol that uses these artifacts as a measuring instrument to quantify cross-model and cross-harness behavioral divergence on a corpus of real skills, together with a static well-formedness analysis. Results tables are scaffolded for population. The intended outcome is to turn "skills do not transfer with the same behavior" from an assertion in the literature into a measured quantity.

## 1. Introduction

The skill format solved a discovery problem: progressive disclosure lets an agent carry many capabilities while loading only what a task needs (Anthropic, 2025). It did not solve a behavioral problem. An activated skill is a prompt, and a prompt is guidance, not enforcement. Whether the skill fires, whether the model follows it, and whether the harness enforces any of it are three separate questions, each answered probabilistically and none answered by the skill file itself.

The consequence is divergence. A survey of the area concludes that despite the open standard, true cross-platform portability remains aspirational (agent-skills survey, 2026), and an analysis of a large skill corpus finds that the same skill behaves inconsistently across agents because the model may ignore the guidance or the skill's assumptions may exceed the model's competence (skill-runtime analysis, 2026). These are qualitative claims. They have not been turned into a measurement that an author can run on a specific skill against a specific set of targets.

The obstacle to measurement is twofold. There is no ground truth for what a skill did unless the harness records a trace, and there is no portable, machine-readable statement of what the skill should have done to compare the trace against. This paper supplies both halves as a methodology and uses them to define an experiment.

Contributions:

1. A formal model of skill behavior as three non-deterministic gates, with realized behavior defined over execution traces (Section 3).
2. A measurement methodology — behavioral contracts evaluated as a deterministic reduction, and three-valued trace alignment — that makes a skill's intended behavior falsifiable and its realized behavior auditable (Section 4).
3. An experimental protocol and metric set for quantifying cross-model and cross-harness divergence and static defect rates, with scaffolded results (Sections 5–6).

We name no implementation. The methodology is presented as a general approach; an instrument that realizes it is out of scope here.

## 2. Background and related work

**Skills and progressive disclosure.** A skill is a directory with a `SKILL.md`; its frontmatter loads at startup, its body on activation, its references on use (Anthropic, 2025). Authoring guidance caps the body at roughly five hundred lines.

**Why behavior degrades.** Instruction adherence falls with instruction density, and earlier instructions are favored over later ones (Jaroslawicz et al., 2026; Elder et al., 2025). Forcing rigid output structure during reasoning degrades reasoning, while offloading exact computation to executable code improves it (Tam et al., 2024; Gao et al., 2022; Chen et al., 2022). Together these motivate a separation between prose, which should carry judgment, and structure, which should carry the parts that must be checked.

**Prior approaches to skill reliability.** Several lines of work are adjacent. A skill-runtime analysis treats skills as code and models as heterogeneous processors, decomposing requirements into primitive capabilities and measuring per model-harness support (skill-runtime analysis, 2026). Contract-based skills for web agents repair an artifact and reuse it after removing the source model (contract-skill study, 2026). A survey catalogs acquisition modes and security challenges and notes that production systems favor human-authored skills for validation and accountability (agent-skills survey, 2026). The intellectual ancestor is design by contract, which specifies obligations and guarantees that a component must satisfy (Meyer, 1992). Our contribution is orthogonal to these: not a runtime, a repair loop, or a compiler, but a measurement methodology that yields an honest three-valued verdict over traces, usable across harnesses without a new execution framework.

## 3. A model of skill behavior

### 3.1 Three gates

Let a skill `s` be installed in an environment with model `m` and harness `h`. Whether `s` affects a task passes three independent gates.

- **Activation** `A(s, m, task)`: whether the model selects `s` as relevant, given only the discovery-layer description. This is a probabilistic routing decision the author does not control and should not pretend to.
- **Adherence** `F(s, m)`: given that `s` is loaded, the degree to which the model follows its instructions. Adherence is partial and order-sensitive.
- **Enforcement** `E(s, h)`: the subset of the skill's intended effects the harness mechanically guarantees, typically empty except where hooks or hard tool boundaries apply.

A skill's effect on a run is the composition of these. The author can specify intent fully but controls only fragments of `E`, influences `F`, and merely hints at `A`. Any honest measurement must treat `A`, `F`, and `E` separately and must not attribute an enforcement guarantee where only adherence is in play.

### 3.2 Intended versus realized behavior

We separate two objects. **Intended behavior** is what the skill should do, expressed by the author. **Realized behavior** is what a run actually did, observable only through the execution trace `τ` that the harness records.

We restrict measurement to the steering decisions that are observable and consequential: which route was selected, in what order routes were preferred, which actions were forbidden, which clarifications were requested, and which declared dependencies were checked. We do not attempt to measure the semantic quality of free-text content; that is the part of the skill that should remain prose, and structure imposed on it would degrade it (Tam et al., 2024).

### 3.3 Decisions as a deterministic reduction

To compare intent against trace, intent must be evaluated deterministically. We model the skill's steering as an ordered set of rules folded into a single decision. Each rule contributes typed effects, combined by fixed operators:

- a preferred route **sets** the current route (last write wins);
- a route ordering **replaces** the current ordering;
- forbidden actions, requested clarifications, and scheduled follow-ups **append**;
- permission grants **merge** into a map;
- a rationale takes the **last** non-empty value.

The fold is deterministic given rule order. It is, deliberately, order-sensitive: because a preferred route sets and an ordering replaces, two individually valid rule sets can compose to different decisions under different concatenation order. This is a property to be tested, not hidden; the contract's test scenarios pin the composed outcome. The reduction is the formal object that makes "what should this skill decide on this input" answerable without invoking a model.

## 4. Methodology

The methodology comprises two artifacts and one analysis.

### 4.1 The behavioral contract

A behavioral contract is a declarative, machine-checkable specification authored alongside a skill. It states the skill's identity, its routes and the rules that steer among them, the clarifications it may request, its declared dependencies and the checks that establish them, the actions it forbids, and a set of falsifiable test scenarios. The contract is not an execution engine; it carries no runtime. It is a specification against which both static well-formedness and dynamic behavior can be judged.

**Static well-formedness.** A contract is well-formed only when a set of invariants holds, checkable without running anything:

- every symbolic reference resolves to a defined identifier (reference closure);
- the dependency-import relation is acyclic, with a valid topological load order;
- no declared resource is orphaned and no on-demand component is unreachable;
- mutually exclusive fields carry exactly one value;
- typed objects reject unknown fields (fail-closed), while explicitly designated open surfaces may carry arbitrary values;
- every test scenario declares at least one concrete expectation.

Well-formedness is a static pre-filter: a contract that fails it is rejected before any execution, the way a type error is caught before runtime.

**Falsifiable tests.** Each test pairs an input with one or more expectations over the decision — the selected route, the route ordering, the forbidden set, the requested clarifications, the matched rules — with include, exact-set, and absence semantics. A test makes a behavioral claim refutable: under model `m`, this input should yield this route. This is the unit that lets behavior, not just structure, be checked, and checked across models rather than asserted for one.

### 4.2 The trace vocabulary

Realized behavior is read from a trace of decision events drawn from a closed vocabulary. The vocabulary records, at minimum: input received, specification loaded, rule evaluated, rule matched, route selected, route order set, action forbidden, permission granted, clarification requested, follow-up scheduled, outcome recorded, and review required. A closed vocabulary is what makes alignment decidable: an event either appears in the trace or it does not, and the absence of a needed event is itself informative.

### 4.3 Trace alignment and the three-valued verdict

Alignment compares the expectations a contract's tests assert against the events a trace records, yielding one of three verdicts per expectation, and an aggregate per skill-run:

- **aligned**: the trace contains evidence that the expectation held;
- **partial**: the trace shows some expectations met and at least one violated;
- **unproven**: the trace lacks the events needed to decide.

The third verdict is the methodological core. A binary pass/fail forces the evaluator to treat missing evidence as either success or failure, and the convenient default — treating the absence of an error as success — is exactly the inference that hides dropped steps. Reporting *unproven* when the trace is insufficient is the difference between measuring behavior and assuming it. It also makes trace completeness a first-class variable: a high unproven rate is a signal about the harness's instrumentation, not only about the skill.

## 5. Experimental protocol

The methodology is used as an instrument to answer three questions: how often do real skills fail static well-formedness; how much does behavior diverge across models and harnesses; and how often is nominal success actually unproven.

**Corpus.** A sample of real skills drawn from public repositories and community catalogs, stratified by size and by whether the skill bundles code. Target initial corpus size and stratification are recorded in Table 2's caption when populated.

**Contract authoring.** For each sampled skill, author a behavioral contract capturing its steering and dependencies, with test scenarios derived from the skill's own stated purpose. Contract authoring is performed independently of the runs to be measured. Author identity and inter-author agreement are reported as a threat-to-validity control.

**Target matrix.** Each skill is run under a matrix of models (at least three distinct families) crossed with harnesses (at least two). Traces are captured using the closed event vocabulary; where a harness does not natively emit an event, that gap is recorded and contributes to the unproven rate rather than being imputed.

**Metrics.**

- **Static defect rate**: fraction of skills whose contracts fail each well-formedness invariant (reference closure, acyclicity, orphan/unreachable, exactly-one, unknown-field).
- **Cross-target alignment distribution**: per model-harness cell, the fraction of test expectations resolving to aligned, partial, and unproven.
- **Divergence**: dispersion of the aligned fraction across cells for the same skill — the quantity that operationalizes "the same skill behaves differently across targets."
- **Unproven-under-success rate**: among runs the harness reports as successful, the fraction of expectations that are unproven — the quantity that operationalizes "success was assumed, not shown."
- **Dependency-declaration completeness**: fraction of skills whose runtime dependencies were fully declared versus discovered only at execution.

## 6. Results

*All tables below are scaffolded. Cells are to be populated as the corpus grows; report counts and confidence intervals where applicable.*

**Table 2. Static well-formedness defects across the corpus.**

| Invariant | Skills failing (n) | % of corpus |
|---|---|---|
| Unresolved reference | [ ] | [ ] |
| Dependency cycle | [ ] | [ ] |
| Orphan / unreachable component | [ ] | [ ] |
| Exactly-one violation | [ ] | [ ] |
| Unknown field (fail-closed) | [ ] | [ ] |
| Test without expectation | [ ] | [ ] |

**Table 3. Cross-target alignment distribution.** Rows are model-harness cells; columns are the fraction of test expectations per verdict. The dispersion of the *aligned* column across rows for the same skill is the divergence metric.

| Model × Harness | Aligned % | Partial % | Unproven % |
|---|---|---|---|
| Model A × Harness 1 | [ ] | [ ] | [ ] |
| Model A × Harness 2 | [ ] | [ ] | [ ] |
| Model B × Harness 1 | [ ] | [ ] | [ ] |
| Model B × Harness 2 | [ ] | [ ] | [ ] |
| Model C × Harness 1 | [ ] | [ ] | [ ] |
| Model C × Harness 2 | [ ] | [ ] | [ ] |

**Table 4. Unproven-under-success.** Among runs reported successful by the harness, the fraction of expectations that remain unproven.

| Model × Harness | Reported-success runs (n) | Unproven expectations % |
|---|---|---|
| Model A × Harness 1 | [ ] | [ ] |
| Model B × Harness 1 | [ ] | [ ] |
| Model C × Harness 1 | [ ] | [ ] |

**Table 5. Dependency-declaration completeness.**

| Declaration status | Skills (n) | % of corpus |
|---|---|---|
| Fully declared | [ ] | [ ] |
| Partially declared | [ ] | [ ] |
| Undeclared (discovered at runtime) | [ ] | [ ] |

## 7. Discussion

If divergence (Table 3 dispersion) is large, the "write once, run anywhere" reading of skill portability is mistaken at the behavioral level even where it holds at the file level, and authors who target a single model are shipping behavior they have not verified for the others. If the unproven-under-success rate (Table 4) is non-trivial, then a meaningful share of "successful" runs are successful only by the convention that absent errors mean success — which is the convention the three-valued verdict exists to refuse. If the static defect rate (Table 2) is non-trivial on real, in-use skills, then a class of failures is catchable before execution by a check that skills do not currently carry.

The methodology's value is that each of these is a number an author can obtain for a specific skill against a specific target set, rather than a general claim. The contract makes intent falsifiable; the trace makes behavior observable; the three-valued verdict keeps the comparison honest.

## 8. Threats to validity

**Contract author bias.** A contract encodes one author's reading of a skill's intent. We mitigate by deriving tests from the skill's stated purpose and by reporting inter-author agreement, but the contract is a model of intent, not intent itself.

**Trace completeness.** Alignment can only prove what the trace records. Harnesses differ in instrumentation, and a high unproven rate may reflect the harness rather than the skill. We treat this as a measured variable, not a confound to be hidden, and report it per cell.

**Open surfaces.** A contract designates some fields as open, carrying arbitrary values — for example a free-form success condition. Behavior expressed only through an open surface is not checkable, and a skill that pushes load-bearing logic into an open surface evades the methodology. The fraction of skills doing so is itself worth reporting.

**Reduction order-sensitivity.** Because the rule fold is order-sensitive, a contract's verdict depends on rule order. This is intended and tested, but it means a contract is a specification of one intended ordering, and re-orderings must be re-tested.

**Corpus representativeness.** Public and community skills may not represent enterprise or generated skills, whose quality distribution differs. Stratification mitigates but does not eliminate this.

## 9. Conclusion

Skills are portable as files and divergent as behavior, and the divergence has been asserted but not measured. We have formalized skill behavior as three non-deterministic gates, proposed a behavioral contract whose decisions reduce deterministically and whose claims are falsifiable, and defined trace alignment with a three-valued verdict that reports *unproven* rather than assuming success. Used as an instrument, these turn cross-target divergence, static defects, and unproven-under-success into quantities an author can measure for a given skill. The empirical tables remain to be populated; the methodology that populates them is the contribution.

## References

Anthropic. (2025). *Equipping agents for the real world with Agent Skills.* Anthropic Engineering.

Anthropic. (2025). *Code execution with MCP: building more efficient AI agents.* Anthropic Engineering.

Chen, W., Ma, X., Wang, X., & Cohen, W. W. (2022). *Program of Thoughts Prompting: Disentangling Computation from Reasoning for Numerical Reasoning Tasks.* arXiv:2211.12588.

Elder, B., Duesterwald, E., & Muthusamy, V. (2025). *Boosting Instruction Following at Scale.* arXiv:2510.14842.

Gao, L., Madaan, A., Zhou, S., Alon, U., Liu, P., Yang, Y., Callan, J., & Neubig, G. (2022). *PAL: Program-aided Language Models.* arXiv:2211.10435.

Jaroslawicz, D., et al. (2026). *How Many Instructions Can LLMs Follow at Once?* arXiv:2507.11538.

Meyer, B. (1992). *Applying "Design by Contract."* IEEE Computer, 25(10), 40–51.

Tam, Z. R., Wu, C.-K., Tsai, Y.-L., Lin, C.-Y., Lee, H.-y., & Chen, Y.-N. (2024). *Let Me Speak Freely? A Study on the Impact of Format Restrictions on Performance of Large Language Models.* arXiv:2408.02442 (EMNLP Findings 2024).

*Sources cited in text by descriptor — agent-skills survey (arXiv:2602.12430), skill-runtime analysis (arXiv:2604.03088), contract-skill study (arXiv:2603.20340) — have author lists and identifiers to be verified before submission.*
