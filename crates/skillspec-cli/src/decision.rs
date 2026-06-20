use crate::model::{Expectation, Predicate, RouteId, Rule, RuleId, ScenarioTest, SkillSpec};
use serde::Serialize;
use std::collections::BTreeMap;

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

    for rule in spec.rules.iter().filter(|rule| matches_rule(rule, input)) {
        apply_rule(&mut decision, rule);
    }

    if decision.route.is_none() {
        decision.route = decision.route_order.first().cloned();
    }

    dedupe_strings(&mut decision.forbid);
    dedupe_strings(&mut decision.elicit);
    dedupe_strings(&mut decision.after_success);
    decision
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

    for expected in &expectation.after_success {
        if !decision
            .after_success
            .iter()
            .any(|actual| actual == expected)
        {
            failures.push(format!("expected after_success {expected:?}"));
        }
    }

    for expected in &expectation.elicit {
        if !decision.elicit.iter().any(|actual| actual == expected) {
            failures.push(format!("expected elicit {expected:?}"));
        }
    }

    failures
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
            .any(|needle| normalized.contains(&needle.to_lowercase()))
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

fn apply_rule(decision: &mut Decision, rule: &Rule) {
    if let Some(route) = &rule.prefer {
        decision.route = Some(route.clone());
    }
    if !rule.route_order.is_empty() {
        decision.route_order = rule.route_order.clone();
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
    .any(|needle| input.contains(needle))
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
    .any(|needle| input.contains(needle))
}

fn long_running(input: &str) -> bool {
    [
        "test", "build", "release", "deploy", "server", "watch", "tail", "monitor",
    ]
    .iter()
    .any(|needle| input.contains(needle))
}

fn interactive(input: &str) -> bool {
    ["login", "auth", "mfa", "otp", "password", "prompt"]
        .iter()
        .any(|needle| input.contains(needle))
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn route_names(routes: &[RouteId]) -> Vec<&str> {
    routes.iter().map(|route| route.0.as_str()).collect()
}
