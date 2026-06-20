use crate::model::{
    Expectation, Predicate, RouteId, Rule, RuleId, ScenarioTest, SkillSpec, TraceEventKind,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct Decision {
    pub input: String,
    pub route: Option<RouteId>,
    pub route_order: Vec<RouteId>,
    pub forbid: Vec<String>,
    pub allow: BTreeMap<String, serde_yaml::Value>,
    pub elicit: Vec<String>,
    pub after_success: Vec<String>,
    pub matched_rules: Vec<MatchedRule>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatchedRule {
    pub id: RuleId,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DecisionEvent {
    InputReceived {
        input: String,
    },
    SpecLoaded {
        skill_id: String,
        schema: String,
    },
    RuleEvaluated {
        rule_id: RuleId,
        matched: bool,
    },
    RuleMatched {
        rule_id: RuleId,
        reason: Option<String>,
    },
    RouteSelected {
        route: RouteId,
    },
    RouteOrderSet {
        route_order: Vec<RouteId>,
    },
    ForbidAdded {
        forbid: Vec<String>,
    },
    AllowAdded {
        allow: BTreeMap<String, serde_yaml::Value>,
    },
    ElicitationRequested {
        elicit: Vec<String>,
    },
    AfterSuccessScheduled {
        after_success: Vec<String>,
    },
    OutcomeRecorded {
        route: Option<RouteId>,
        matched_rules: Vec<RuleId>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct TestRun {
    pub passed: Vec<TestCaseResult>,
    pub failed: Vec<TestCaseResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TestCaseResult {
    pub name: String,
    pub input: String,
    pub failures: Vec<String>,
    pub decision: Decision,
}

pub fn decide(spec: &SkillSpec, input: &str) -> Decision {
    decide_with_events(spec, input).decision
}

pub fn decide_with_events(spec: &SkillSpec, input: &str) -> DecisionWithEvents {
    let mut decision = Decision {
        input: input.to_owned(),
        route: None,
        route_order: default_route_order(spec),
        forbid: Vec::new(),
        allow: BTreeMap::new(),
        elicit: Vec::new(),
        after_success: Vec::new(),
        matched_rules: Vec::new(),
        reason: None,
    };
    let mut events = vec![
        DecisionEvent::InputReceived {
            input: input.to_owned(),
        },
        DecisionEvent::SpecLoaded {
            skill_id: spec.id.clone(),
            schema: spec.schema.clone(),
        },
    ];

    for rule in &spec.rules {
        let matched = matches_rule(rule, input);
        events.push(DecisionEvent::RuleEvaluated {
            rule_id: rule.id.clone(),
            matched,
        });
        if matched {
            apply_rule(&mut decision, &mut events, rule);
        }
    }

    if decision.route.is_none() {
        decision.route = decision.route_order.first().cloned();
        if let Some(route) = &decision.route {
            events.push(DecisionEvent::RouteSelected {
                route: route.clone(),
            });
        }
    }

    dedupe_strings(&mut decision.forbid);
    dedupe_strings(&mut decision.elicit);
    dedupe_strings(&mut decision.after_success);
    events.push(DecisionEvent::OutcomeRecorded {
        route: decision.route.clone(),
        matched_rules: decision
            .matched_rules
            .iter()
            .map(|matched| matched.id.clone())
            .collect(),
    });

    DecisionWithEvents { decision, events }
}

#[derive(Clone, Debug, Serialize)]
pub struct DecisionWithEvents {
    pub decision: Decision,
    pub events: Vec<DecisionEvent>,
}

pub fn run_tests(spec: &SkillSpec) -> TestRun {
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    for test in &spec.tests {
        let result = run_test(spec, test);
        if result.failures.is_empty() {
            passed.push(result);
        } else {
            failed.push(result);
        }
    }

    TestRun { passed, failed }
}

fn run_test(spec: &SkillSpec, test: &ScenarioTest) -> TestCaseResult {
    let decision = decide(spec, &test.input);
    let failures = compare_expectation(&decision, &test.expect);
    TestCaseResult {
        name: test.name.clone(),
        input: test.input.clone(),
        failures,
        decision,
    }
}

fn compare_expectation(decision: &Decision, expectation: &Expectation) -> Vec<String> {
    let mut failures = Vec::new();

    if let Some(expected_route) = &expectation.route {
        if decision.route.as_ref() != Some(expected_route) {
            failures.push(format!(
                "expected route {}, got {}",
                expected_route.0,
                decision
                    .route
                    .as_ref()
                    .map(|route| route.0.as_str())
                    .unwrap_or("<none>")
            ));
        }
    }

    if !expectation.route_order.is_empty() && decision.route_order != expectation.route_order {
        failures.push(format!(
            "expected route_order {:?}, got {:?}",
            route_names(&expectation.route_order),
            route_names(&decision.route_order)
        ));
    }

    for expected in &expectation.forbid {
        if !decision.forbid.iter().any(|actual| actual == expected) {
            failures.push(format!("expected forbid {expected:?}"));
        }
    }
    if let Some(expected) = &expectation.forbid_exact {
        compare_string_set("forbid_exact", expected, &decision.forbid, &mut failures);
    }
    for expected in &expectation.not_forbid {
        if decision.forbid.iter().any(|actual| actual == expected) {
            failures.push(format!("expected forbid not to contain {expected:?}"));
        }
    }

    for expected in &expectation.after_success {
        if !decision
            .after_success
            .iter()
            .any(|actual| actual == expected)
        {
            failures.push(format!("expected after_success {expected:?}"));
        }
    }
    if let Some(expected) = &expectation.after_success_exact {
        compare_string_set(
            "after_success_exact",
            expected,
            &decision.after_success,
            &mut failures,
        );
    }
    for expected in &expectation.not_after_success {
        if decision
            .after_success
            .iter()
            .any(|actual| actual == expected)
        {
            failures.push(format!(
                "expected after_success not to contain {expected:?}"
            ));
        }
    }

    for expected in &expectation.elicit {
        if !decision.elicit.iter().any(|actual| actual == expected) {
            failures.push(format!("expected elicit {expected:?}"));
        }
    }
    if let Some(expected) = &expectation.elicit_exact {
        compare_string_set("elicit_exact", expected, &decision.elicit, &mut failures);
    }
    for expected in &expectation.not_elicit {
        if decision.elicit.iter().any(|actual| actual == expected) {
            failures.push(format!("expected elicit not to contain {expected:?}"));
        }
    }

    let matched_rules = decision
        .matched_rules
        .iter()
        .map(|matched| matched.id.clone())
        .collect::<Vec<_>>();
    for expected in &expectation.matched_rules {
        if !matched_rules.iter().any(|actual| actual == expected) {
            failures.push(format!("expected matched_rule {:?}", expected.0));
        }
    }
    if let Some(expected) = &expectation.matched_rules_exact {
        compare_rule_set(
            "matched_rules_exact",
            expected,
            &matched_rules,
            &mut failures,
        );
    }
    for expected in &expectation.not_matched_rules {
        if matched_rules.iter().any(|actual| actual == expected) {
            failures.push(format!(
                "expected matched_rules not to contain {:?}",
                expected.0
            ));
        }
    }

    failures
}

fn compare_string_set(
    label: &str,
    expected: &[String],
    actual: &[String],
    failures: &mut Vec<String>,
) {
    let expected = expected.iter().cloned().collect::<BTreeSet<_>>();
    let actual = actual.iter().cloned().collect::<BTreeSet<_>>();
    if expected != actual {
        failures.push(format!("expected {label} {:?}, got {:?}", expected, actual));
    }
}

fn compare_rule_set(
    label: &str,
    expected: &[RuleId],
    actual: &[RuleId],
    failures: &mut Vec<String>,
) {
    let expected = expected
        .iter()
        .map(|rule| rule.0.clone())
        .collect::<BTreeSet<_>>();
    let actual = actual
        .iter()
        .map(|rule| rule.0.clone())
        .collect::<BTreeSet<_>>();
    if expected != actual {
        failures.push(format!("expected {label} {:?}, got {:?}", expected, actual));
    }
}

fn matches_rule(rule: &Rule, input: &str) -> bool {
    matches_predicate(&rule.when, input)
}

fn matches_predicate(predicate: &Predicate, input: &str) -> bool {
    let normalized = input.to_lowercase();
    let mut has_condition = false;

    if !predicate.user_says_any.is_empty() {
        has_condition = true;
        if !predicate
            .user_says_any
            .iter()
            .any(|needle| contains_phrase(&normalized, &needle.to_lowercase()))
        {
            return false;
        }
    }

    if let Some(expected) = predicate.task_recurrence_likely {
        has_condition = true;
        if recurrence_likely(&normalized) != expected {
            return false;
        }
    }

    if let Some(expected) = predicate.domain_object_task {
        has_condition = true;
        if domain_object_task(&normalized) != expected {
            return false;
        }
    }

    if let Some(expected) = predicate.command_likely_long_running {
        has_condition = true;
        if long_running(&normalized) != expected {
            return false;
        }
    }

    if let Some(expected) = predicate.interactive_prompt_likely {
        has_condition = true;
        if interactive(&normalized) != expected {
            return false;
        }
    }

    has_condition
}

fn apply_rule(decision: &mut Decision, events: &mut Vec<DecisionEvent>, rule: &Rule) {
    events.push(DecisionEvent::RuleMatched {
        rule_id: rule.id.clone(),
        reason: rule.reason.clone(),
    });
    if let Some(route) = &rule.prefer {
        decision.route = Some(route.clone());
        events.push(DecisionEvent::RouteSelected {
            route: route.clone(),
        });
    }
    if !rule.route_order.is_empty() {
        decision.route_order = rule.route_order.clone();
        events.push(DecisionEvent::RouteOrderSet {
            route_order: rule.route_order.clone(),
        });
    }
    if !rule.forbid.is_empty() {
        events.push(DecisionEvent::ForbidAdded {
            forbid: rule.forbid.clone(),
        });
    }
    if !rule.allow.is_empty() {
        events.push(DecisionEvent::AllowAdded {
            allow: rule.allow.clone(),
        });
    }
    if !rule.elicit.is_empty() {
        events.push(DecisionEvent::ElicitationRequested {
            elicit: rule.elicit.clone(),
        });
    }
    if !rule.after_success.is_empty() {
        events.push(DecisionEvent::AfterSuccessScheduled {
            after_success: rule.after_success.clone(),
        });
    }
    decision.forbid.extend(rule.forbid.clone());
    decision.elicit.extend(rule.elicit.clone());
    decision.after_success.extend(rule.after_success.clone());
    decision.allow.extend(rule.allow.clone());
    decision.reason = rule.reason.clone().or_else(|| decision.reason.clone());
    decision.matched_rules.push(MatchedRule {
        id: rule.id.clone(),
        reason: rule.reason.clone(),
    });
}

impl DecisionEvent {
    pub fn kind(&self) -> TraceEventKind {
        match self {
            Self::InputReceived { .. } => TraceEventKind::InputReceived,
            Self::SpecLoaded { .. } => TraceEventKind::SpecLoaded,
            Self::RuleEvaluated { .. } => TraceEventKind::RuleEvaluated,
            Self::RuleMatched { .. } => TraceEventKind::RuleMatched,
            Self::RouteSelected { .. } => TraceEventKind::RouteSelected,
            Self::RouteOrderSet { .. } => TraceEventKind::RouteOrderSet,
            Self::ForbidAdded { .. } => TraceEventKind::ForbidAdded,
            Self::AllowAdded { .. } => TraceEventKind::AllowAdded,
            Self::ElicitationRequested { .. } => TraceEventKind::ElicitationRequested,
            Self::AfterSuccessScheduled { .. } => TraceEventKind::AfterSuccessScheduled,
            Self::OutcomeRecorded { .. } => TraceEventKind::OutcomeRecorded,
        }
    }

    pub fn name(&self) -> &'static str {
        match self.kind() {
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
}

fn default_route_order(spec: &SkillSpec) -> Vec<RouteId> {
    let mut routes = spec.routes.clone();
    routes.sort_by_key(|route| route.rank.unwrap_or(i64::MAX));
    routes.into_iter().map(|route| route.id).collect()
}

fn recurrence_likely(input: &str) -> bool {
    [
        "daily",
        "weekly",
        "monthly",
        "every morning",
        "every day",
        "every week",
        "every month",
        "again",
        "recurring",
    ]
    .iter()
    .any(|needle| contains_phrase(input, needle))
}

fn domain_object_task(input: &str) -> bool {
    [
        "alert",
        "ticket",
        "issue",
        "calendar",
        "email",
        "crm",
        "repo",
        "pull request",
        "dashboard",
        "invoice",
        "customer",
        "incident",
    ]
    .iter()
    .any(|needle| contains_phrase(input, needle))
}

fn long_running(input: &str) -> bool {
    [
        "test", "build", "release", "deploy", "server", "watch", "tail", "monitor",
    ]
    .iter()
    .any(|needle| contains_phrase(input, needle))
}

fn interactive(input: &str) -> bool {
    ["login", "auth", "mfa", "otp", "password", "prompt"]
        .iter()
        .any(|needle| contains_phrase(input, needle))
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn route_names(routes: &[RouteId]) -> Vec<&str> {
    routes.iter().map(|route| route.0.as_str()).collect()
}

fn contains_phrase(input: &str, phrase: &str) -> bool {
    let phrase = phrase.trim();
    if phrase.is_empty() {
        return false;
    }
    bounded_contains(input, phrase)
        || plural_candidate(phrase).is_some_and(|plural| bounded_contains(input, &plural))
}

fn bounded_contains(input: &str, phrase: &str) -> bool {
    let mut search_start = 0;
    while let Some(offset) = input[search_start..].find(phrase) {
        let start = search_start + offset;
        let end = start + phrase.len();
        if phrase_boundary(input, start, end) {
            return true;
        }
        search_start = end;
        if search_start >= input.len() {
            return false;
        }
    }
    false
}

fn plural_candidate(phrase: &str) -> Option<String> {
    if phrase.ends_with('s') {
        None
    } else {
        Some(format!("{phrase}s"))
    }
}

fn phrase_boundary(input: &str, start: usize, end: usize) -> bool {
    let before = input[..start].chars().next_back();
    let after = input[end..].chars().next();
    !before.is_some_and(is_identifier_char) && !after.is_some_and(is_identifier_char)
}

fn is_identifier_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_tests_support_exact_negative_and_matched_rule_expectations() {
        let yaml = r#"
schema: skillspec/v0
id: decision.expectations
title: Decision Expectations
description: Exercises richer scenario assertions.
routes:
  - id: local
    label: Local
    rank: 10
  - id: browser
    label: Browser
    rank: 20
rules:
  - id: browse_rule
    when:
      user_says_any: ["browse"]
    prefer: browser
    forbid: [native_search_as_answer]
    reason: Browser requests must use browser route.
tests:
  - name: browse selects browser exactly
    input: browse the page
    expect:
      route: browser
      forbid_exact: [native_search_as_answer]
      not_forbid: [raw_shell]
      elicit_exact: []
      after_success_exact: []
      matched_rules_exact: [browse_rule]
      not_matched_rules: []
"#;
        let spec = serde_yaml::from_str::<SkillSpec>(yaml).unwrap();
        crate::parser::validate_spec(&spec).unwrap();

        let result = run_tests(&spec);

        assert_eq!(result.passed.len(), 1);
        assert!(result.failed.is_empty());
    }
}
