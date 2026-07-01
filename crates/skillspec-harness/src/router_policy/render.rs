use super::model::{
    PolicyGetReport, PolicyInitReport, PolicyListReport, PolicyRemoveRuleReport,
    PolicySetProfileReport, PolicySetRuleReport, PolicyShowReport, ProfileApplyReport,
    ProfileClearReport, ProfileStatusReport,
};

pub fn render_init(report: &PolicyInitReport) -> String {
    format!(
        "Skill router policy\n\nIndex: {}\nInitialized: {}\n",
        report.index.display(),
        report.initialized
    )
}

pub fn render_list(report: &PolicyListReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router policy profiles\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    if report.profiles.is_empty() {
        output.push_str("Profiles: none\n");
        return output;
    }
    output.push_str("Profiles:\n");
    for profile in &report.profiles {
        let active = if profile.active { " active" } else { "" };
        output.push_str(&format!(
            "- {} [{}{}]\n",
            profile.name,
            profile.mode.as_str(),
            active
        ));
    }
    output
}

pub fn render_show(report: &PolicyShowReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router policy\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    if let Some(profile) = &report.profile {
        output.push_str(&format!("Profile: {}\n", profile.name));
        output.push_str(&format!("Mode: {}\n", profile.mode.as_str()));
        output.push_str(&format!("Active: {}\n", profile.active));
    } else {
        output.push_str("Profile: none\n");
    }
    if report.rules.is_empty() {
        output.push_str("Rules: none\n");
        return output;
    }
    output.push_str("\nRules:\n");
    for rule in &report.rules {
        output.push_str(&format!(
            "- {} priority={} mode={} anchor={} enabled={}\n",
            rule.id,
            rule.priority,
            rule.mode.as_str(),
            rule.anchor.as_str(),
            rule.enabled
        ));
        for preference in &rule.preferences {
            output.push_str(&format!(
                "  {} {}:{} weight={:.1}\n",
                preference.effect.as_str(),
                preference.target_kind.as_str(),
                preference.target_value,
                preference.weight.unwrap_or(0.0)
            ));
        }
    }
    output
}

pub fn render_get(report: &PolicyGetReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router policy item\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Id: {}\n", report.id));
    if let Some(profile) = &report.profile {
        output.push_str(&format!("Profile: {}\n", profile.name));
        output.push_str(&format!("Mode: {}\n", profile.mode.as_str()));
        output.push_str(&format!("Active: {}\n", profile.active));
    }
    if !report.rules.is_empty() {
        output.push_str("\nRules:\n");
        for rule in &report.rules {
            output.push_str(&format!(
                "- {} profile={} priority={} mode={} anchor={} enabled={}\n",
                rule.id,
                rule.profile,
                rule.priority,
                rule.mode.as_str(),
                rule.anchor.as_str(),
                rule.enabled
            ));
        }
    }
    output
}

pub fn render_set_profile(report: &PolicySetProfileReport) -> String {
    let active = report
        .active_profile
        .as_deref()
        .map_or("none", |profile| profile);
    format!(
        "Skill router policy profile\n\nIndex: {}\nProfile: {}\nMode: {}\nActive profile: {}\n",
        report.index.display(),
        report.profile.name,
        report.profile.mode.as_str(),
        active
    )
}

pub fn render_set_rule(report: &PolicySetRuleReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router policy rule\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Rule: {}\n", report.rule.id));
    output.push_str(&format!("Profile: {}\n", report.rule.profile));
    output.push_str(&format!("Priority: {}\n", report.rule.priority));
    if !report.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

pub fn render_remove_rule(report: &PolicyRemoveRuleReport) -> String {
    format!(
        "Skill router policy rule\n\nIndex: {}\nRule: {}\nRemoved: {}\n",
        report.index.display(),
        report.id,
        report.removed
    )
}

pub fn render_profile_status(report: &ProfileStatusReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router profile\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    if let Some(profile) = &report.active_profile {
        output.push_str(&format!("Active profile: {}\n", profile.name));
        output.push_str(&format!("Mode: {}\n", profile.mode.as_str()));
    } else {
        output.push_str("Active profile: none\n");
    }
    output
}

pub fn render_profile_apply(report: &ProfileApplyReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router profile apply\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Profile: {}\n", report.profile));
    output.push_str(&format!("Mode: {}\n", report.mode.as_str()));
    output.push_str(&format!("Applied: {}\n", report.applied));
    if !report.notes.is_empty() {
        output.push_str("\nNotes:\n");
        for note in &report.notes {
            output.push_str(&format!("- {note}\n"));
        }
    }
    output
}

pub fn render_profile_clear(report: &ProfileClearReport) -> String {
    format!(
        "Skill router profile clear\n\nIndex: {}\nCleared: {}\nDry run: {}\n",
        report.index.display(),
        report.cleared,
        report.dry_run
    )
}
