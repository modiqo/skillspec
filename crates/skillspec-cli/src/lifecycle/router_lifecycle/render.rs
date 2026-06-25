use super::*;

pub fn render_install(report: &RouterInstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router install\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
    if report.router_skill_dirs.len() > 1 {
        output.push_str("Router roots:\n");
        for router_skill in &report.router_skill_reports {
            output.push_str(&format!(
                "- {} ({:?})\n",
                router_skill.path.display(),
                router_skill.status
            ));
        }
    }
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!(
        "Durable executor: {}\n",
        report.durable_executor.message
    ));
    if !report.durable_executor.warnings.is_empty() {
        output.push_str("Durable warnings:\n");
        for warning in &report.durable_executor.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output.push_str(&format!(
        "Visibility changes: {}\n",
        report.visibility.changes.len()
    ));
    if let Some(index_report) = &report.index_report {
        output.push_str(&format!(
            "Skills indexed: {}\n",
            index_report.skills_indexed
        ));
    }
    output.push_str(&format!("Prepared: {}\n", report.preparedness.ready));
    output.push_str(&format!(
        "Index stale after build: {}\n",
        report.preparedness.index_stale
    ));
    output
}

pub fn render_refresh(report: &RouterRefreshReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router index refresh\n\n");
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!(
        "Router config present: {}\n",
        report.router_config_present
    ));
    if let Some(config) = &report.config {
        output.push_str(&format!("Config: {}\n", config.display()));
    }
    if let Some(manifest) = &report.visibility_manifest {
        output.push_str(&format!("Manifest: {}\n", manifest.display()));
    }
    output.push_str(&format!(
        "Detected stale before refresh: {}\n",
        report.status_before.stale
    ));
    output.push_str(&format!(
        "Visibility changes: {}\n",
        report
            .visibility
            .as_ref()
            .map(|visibility| visibility.changes.len())
            .unwrap_or(0)
    ));
    output.push_str(&format!(
        "Skills indexed: {}\n",
        report.index_report.skills_indexed
    ));
    if let Some(preparedness) = &report.preparedness {
        output.push_str(&format!("Prepared: {}\n", preparedness.ready));
        output.push_str(&format!(
            "Index stale after build: {}\n",
            preparedness.index_stale
        ));
    }
    if !report.advice.is_empty() {
        output.push_str("\nAdvice:\n");
        for advice in &report.advice {
            output.push_str(&format!("- {advice}\n"));
        }
    }
    output
}

pub fn render_update(report: &RouterUpdateReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router update\n\n");
    output.push_str("Router roots:\n");
    for router_skill in &report.router_skill_reports {
        output.push_str(&format!(
            "- {} ({:?})\n",
            router_skill.path.display(),
            router_skill.status
        ));
    }
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    if let Some(backup) = &report.backup {
        output.push_str(&format!("Backup: {}\n", backup.path.display()));
        output.push_str(&format!("Backup items: {}\n", backup.items.len()));
    }
    output.push_str(&format!(
        "Durable executor: {}\n",
        report.durable_executor.message
    ));
    if let Some(index_report) = &report.index_report {
        output.push_str(&format!(
            "Skills indexed: {}\n",
            index_report.skills_indexed
        ));
    }
    if let Some(preparedness) = &report.preparedness {
        output.push_str(&format!("Prepared: {}\n", preparedness.ready));
        output.push_str(&format!(
            "Index stale after update: {}\n",
            preparedness.index_stale
        ));
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

pub fn render_mode(report: &RouterModeReport) -> String {
    let mut output = String::new();
    let action = if report.enabled { "enable" } else { "disable" };
    output.push_str(&format!("Skill router {action}\n\n"));
    output.push_str("Router roots:\n");
    for router_skill in &report.router_skill_reports {
        output.push_str(&format!(
            "- {} ({:?})\n",
            router_skill.path.display(),
            router_skill.status
        ));
    }
    output.push_str(&format!("Index: {}\n", report.index.display()));
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    output.push_str(&format!("Config: {}\n", report.config.display()));
    output.push_str(&format!("Enabled: {}\n", report.enabled));
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!(
        "Durable executor: {}\n",
        report.durable_executor.message
    ));
    output.push_str(&format!(
        "Visibility changes: {}\n",
        report.visibility.changes.len()
    ));
    if let Some(index_report) = &report.index_report {
        output.push_str(&format!(
            "Skills indexed: {}\n",
            index_report.skills_indexed
        ));
    }
    if let Some(preparedness) = &report.preparedness {
        output.push_str(&format!("Prepared: {}\n", preparedness.ready));
        output.push_str(&format!(
            "Index stale after enable: {}\n",
            preparedness.index_stale
        ));
    }
    output.push_str(&format!("Restart warning: {}\n", report.restart_warning));
    output
}

pub fn render_uninstall(report: &RouterUninstallReport) -> String {
    let mut output = String::new();
    output.push_str("Skill router uninstall\n\n");
    output.push_str(&format!("Router: {}\n", report.router_skill_dir.display()));
    if report.router_skill_dirs.len() > 1 {
        output.push_str("Router roots:\n");
        for router_skill in &report.router_skill_reports {
            output.push_str(&format!(
                "- {} ({:?})\n",
                router_skill.path.display(),
                router_skill.status
            ));
        }
    }
    output.push_str(&format!("Manifest: {}\n", report.manifest.display()));
    if let Some(index) = &report.index {
        output.push_str(&format!("Index: {}\n", index.display()));
    }
    output.push_str(&format!("Dry run: {}\n", report.dry_run));
    output.push_str(&format!(
        "Visibility restores: {}\n",
        report.restore.changes.len()
    ));
    output.push_str(&format!("Index removed: {}\n", report.index_removed));
    output
}
