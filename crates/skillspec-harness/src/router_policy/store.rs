use rusqlite::{params, Connection, OptionalExtension};
use skillspec_core::error::{Error, Result};
use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use super::model::{
    PolicyAnchor, PolicyEffect, PolicyGetOptions, PolicyGetReport, PolicyInitOptions,
    PolicyInitReport, PolicyListOptions, PolicyListReport, PolicyPredicatesReport,
    PolicyPreferenceReport, PolicyProfileMode, PolicyProfileReport, PolicyRemoveRuleOptions,
    PolicyRemoveRuleReport, PolicyRuleMode, PolicyRuleReport, PolicySetProfileOptions,
    PolicySetProfileReport, PolicySetRuleOptions, PolicySetRuleReport, PolicyShowOptions,
    PolicyShowReport, PolicyTargetKind, ProfileApplyOptions, ProfileApplyReport,
    ProfileClearOptions, ProfileClearReport, ProfileStatusOptions, ProfileStatusReport,
};
use super::target::{parse_targets, PolicyTarget};

pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS router_policy_profiles (
          name TEXT PRIMARY KEY,
          mode TEXT NOT NULL,
          strict INTEGER NOT NULL DEFAULT 0,
          default_decision TEXT,
          active INTEGER NOT NULL DEFAULT 0,
          description TEXT,
          updated_at_unix INTEGER
        );

        CREATE TABLE IF NOT EXISTS router_policy_rules (
          id TEXT PRIMARY KEY,
          profile TEXT NOT NULL,
          priority INTEGER NOT NULL DEFAULT 0,
          mode TEXT NOT NULL DEFAULT 'soft',
          anchor TEXT NOT NULL DEFAULT 'none',
          ordinal INTEGER NOT NULL,
          enabled INTEGER NOT NULL DEFAULT 1,
          FOREIGN KEY(profile) REFERENCES router_policy_profiles(name)
        );

        CREATE TABLE IF NOT EXISTS router_policy_predicates (
          rule_id TEXT NOT NULL,
          kind TEXT NOT NULL,
          phrase TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          FOREIGN KEY(rule_id) REFERENCES router_policy_rules(id)
        );

        CREATE TABLE IF NOT EXISTS router_policy_preferences (
          rule_id TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          effect TEXT NOT NULL,
          target_kind TEXT NOT NULL,
          target_value TEXT NOT NULL,
          weight REAL,
          FOREIGN KEY(rule_id) REFERENCES router_policy_rules(id)
        );

        CREATE TABLE IF NOT EXISTS router_policy_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          at_unix INTEGER NOT NULL,
          actor TEXT,
          action TEXT NOT NULL,
          profile TEXT,
          summary TEXT
        );

        CREATE INDEX IF NOT EXISTS router_policy_rules_profile_priority
          ON router_policy_rules(profile, enabled, priority DESC, ordinal ASC);
        CREATE INDEX IF NOT EXISTS router_policy_predicates_rule_kind
          ON router_policy_predicates(rule_id, kind);
        CREATE INDEX IF NOT EXISTS router_policy_preferences_rule_order
          ON router_policy_preferences(rule_id, effect, ordinal);
        "#,
    )?;
    Ok(())
}

pub fn init(options: PolicyInitOptions) -> Result<PolicyInitReport> {
    let index = super::normalize_index_path(options.index.clone());
    let conn = Connection::open(&index)?;
    create_schema(&conn)?;
    Ok(PolicyInitReport {
        index,
        initialized: true,
    })
}

pub fn list(options: PolicyListOptions) -> Result<PolicyListReport> {
    let index = super::normalize_index_path(options.index.clone());
    let conn = Connection::open(&index)?;
    create_schema(&conn)?;
    Ok(PolicyListReport {
        index,
        profiles: list_profiles(&conn)?,
    })
}

pub fn show(options: PolicyShowOptions) -> Result<PolicyShowReport> {
    let index = super::normalize_index_path(options.index.clone());
    let conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let profile = match &options.profile {
        Some(name) => read_profile(&conn, name)?,
        None => read_active_profile(&conn)?,
    };
    let rules = if let Some(profile) = &profile {
        list_rules(&conn, Some(&profile.name))?
    } else {
        list_rules(&conn, None)?
    };
    Ok(PolicyShowReport {
        index,
        profile,
        rules,
    })
}

pub fn get(options: PolicyGetOptions) -> Result<PolicyGetReport> {
    let index = super::normalize_index_path(options.index.clone());
    let conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let profile = read_profile(&conn, &options.id)?;
    let mut rules = Vec::new();
    if let Some(profile) = &profile {
        rules.extend(list_rules(&conn, Some(&profile.name))?);
    }
    if let Some(rule) = read_rule(&conn, &options.id)? {
        rules.push(rule);
    }
    if profile.is_none() && rules.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "router policy profile or rule {:?} does not exist",
                options.id
            ),
        });
    }
    Ok(PolicyGetReport {
        index,
        id: options.id,
        profile,
        rules,
    })
}

pub fn set_profile(options: PolicySetProfileOptions) -> Result<PolicySetProfileReport> {
    validate_name("profile", &options.name)?;
    let index = super::normalize_index_path(options.index.clone());
    let mut conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let tx = conn.transaction()?;
    if options.active {
        tx.execute("UPDATE router_policy_profiles SET active = 0", [])?;
    }
    tx.execute(
        r#"
        INSERT INTO router_policy_profiles
          (name, mode, strict, default_decision, active, description, updated_at_unix)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(name) DO UPDATE SET
          mode = excluded.mode,
          strict = excluded.strict,
          default_decision = excluded.default_decision,
          active = CASE WHEN excluded.active = 1 THEN 1 ELSE router_policy_profiles.active END,
          description = excluded.description,
          updated_at_unix = excluded.updated_at_unix
        "#,
        params![
            options.name,
            options.mode.as_str(),
            options.strict,
            default_decision(options.mode),
            options.active,
            options.description,
            now_unix()
        ],
    )?;
    if options.active {
        tx.execute(
            "UPDATE router_policy_profiles SET active = CASE WHEN name = ?1 THEN 1 ELSE 0 END",
            params![options.name],
        )?;
    }
    record_event(
        &tx,
        "set-profile",
        Some(&options.name),
        &format!("mode={}", options.mode.as_str()),
    )?;
    tx.commit()?;
    let profile = read_profile(&conn, &options.name)?.ok_or_else(|| Error::InvalidInput {
        message: format!("profile {:?} was not stored", options.name),
    })?;
    let active_profile = read_active_profile(&conn)?.map(|profile| profile.name);
    Ok(PolicySetProfileReport {
        index,
        profile,
        active_profile,
    })
}

pub fn set_rule(options: PolicySetRuleOptions) -> Result<PolicySetRuleReport> {
    validate_name("rule", &options.id)?;
    validate_name("profile", &options.profile)?;
    let index = super::normalize_index_path(options.index.clone());
    let mut conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let profile = require_profile(&conn, &options.profile)?;
    let preferences = build_preferences(&options)?;
    if preferences.is_empty() {
        return Err(Error::InvalidInput {
            message: "policy rule must include at least one --prefer, --allow, --suppress, or --forbid target".to_owned(),
        });
    }
    let warnings = policy_warnings(&conn, &options.profile, &preferences)?;
    if profile.strict && !warnings.is_empty() {
        return Err(Error::InvalidInput {
            message: format!(
                "strict router policy profile {:?} rejected rule {:?}: {}",
                profile.name,
                options.id,
                warnings.join("; ")
            ),
        });
    }
    let tx = conn.transaction()?;
    let ordinal = next_rule_ordinal(&tx, &options.profile, &options.id)?;
    tx.execute(
        r#"
        INSERT INTO router_policy_rules
          (id, profile, priority, mode, anchor, ordinal, enabled)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(id) DO UPDATE SET
          profile = excluded.profile,
          priority = excluded.priority,
          mode = excluded.mode,
          anchor = excluded.anchor,
          enabled = excluded.enabled
        "#,
        params![
            &options.id,
            &options.profile,
            options.priority,
            options.mode.as_str(),
            options.anchor.as_str(),
            ordinal,
            options.enabled
        ],
    )?;
    tx.execute(
        "DELETE FROM router_policy_predicates WHERE rule_id = ?1",
        params![&options.id],
    )?;
    tx.execute(
        "DELETE FROM router_policy_preferences WHERE rule_id = ?1",
        params![&options.id],
    )?;
    insert_predicates(&tx, &options.id, "any_keywords", &options.when_any)?;
    insert_predicates(&tx, &options.id, "all_keywords", &options.when_all)?;
    insert_predicates(&tx, &options.id, "none_keywords", &options.when_none)?;
    for (ordinal, preference) in preferences.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO router_policy_preferences
              (rule_id, ordinal, effect, target_kind, target_value, weight)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                &options.id,
                ordinal as i64,
                preference.effect.as_str(),
                preference.target.kind.as_str(),
                preference.target.value,
                preference.weight
            ],
        )?;
    }
    record_event(
        &tx,
        "set-rule",
        Some(&options.profile),
        &format!("rule={}", options.id),
    )?;
    tx.commit()?;
    let rule = read_rule(&conn, &options.id)?.ok_or_else(|| Error::InvalidInput {
        message: format!("policy rule {:?} was not stored", options.id),
    })?;
    Ok(PolicySetRuleReport {
        index,
        rule,
        warnings,
    })
}

pub fn remove_rule(options: PolicyRemoveRuleOptions) -> Result<PolicyRemoveRuleReport> {
    let index = super::normalize_index_path(options.index.clone());
    let mut conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM router_policy_predicates WHERE rule_id = ?1",
        params![options.id],
    )?;
    tx.execute(
        "DELETE FROM router_policy_preferences WHERE rule_id = ?1",
        params![options.id],
    )?;
    let removed = tx.execute(
        "DELETE FROM router_policy_rules WHERE id = ?1",
        params![options.id],
    )? > 0;
    record_event(&tx, "remove-rule", None, &format!("rule={}", options.id))?;
    tx.commit()?;
    Ok(PolicyRemoveRuleReport {
        index,
        removed,
        id: options.id,
    })
}

pub fn profile_status(options: ProfileStatusOptions) -> Result<ProfileStatusReport> {
    let index = super::normalize_index_path(options.index.clone());
    let conn = Connection::open(&index)?;
    create_schema(&conn)?;
    Ok(ProfileStatusReport {
        index,
        active_profile: read_active_profile(&conn)?,
    })
}

pub fn profile_apply(options: ProfileApplyOptions) -> Result<ProfileApplyReport> {
    let index = super::normalize_index_path(options.index.clone());
    let mut conn = Connection::open(&index)?;
    create_schema(&conn)?;
    let Some(profile) = read_profile(&conn, &options.profile)? else {
        return Err(Error::InvalidInput {
            message: format!("router policy profile {:?} does not exist", options.profile),
        });
    };
    if !options.dry_run {
        let tx = conn.transaction()?;
        tx.execute("UPDATE router_policy_profiles SET active = 0", [])?;
        tx.execute(
            "UPDATE router_policy_profiles SET active = 1, updated_at_unix = ?2 WHERE name = ?1",
            params![options.profile, now_unix()],
        )?;
        record_event(&tx, "activate-profile", Some(&options.profile), "active=1")?;
        tx.commit()?;
    }
    let mut notes = Vec::new();
    if profile.mode == PolicyProfileMode::NativePassthrough {
        notes.push(
            "native-passthrough is recorded as active policy state; harness visibility mutation is not performed by this command yet".to_owned(),
        );
    }
    Ok(ProfileApplyReport {
        index,
        profile: options.profile,
        applied: !options.dry_run,
        dry_run: options.dry_run,
        mode: profile.mode,
        notes,
    })
}

pub fn profile_clear(options: ProfileClearOptions) -> Result<ProfileClearReport> {
    let index = super::normalize_index_path(options.index.clone());
    let mut conn = Connection::open(&index)?;
    create_schema(&conn)?;
    if !options.dry_run {
        let tx = conn.transaction()?;
        tx.execute("UPDATE router_policy_profiles SET active = 0", [])?;
        record_event(&tx, "clear-profile", None, "active=0")?;
        tx.commit()?;
    }
    Ok(ProfileClearReport {
        index,
        cleared: !options.dry_run,
        dry_run: options.dry_run,
    })
}

pub(crate) fn read_active_policy(conn: &Connection) -> Result<Option<ActivePolicy>> {
    create_schema(conn)?;
    let Some(profile) = read_active_profile(conn)? else {
        return Ok(None);
    };
    let rules = list_active_rules(conn, &profile.name)?;
    Ok(Some(ActivePolicy { profile, rules }))
}

pub(crate) fn read_named_policy(conn: &Connection, name: &str) -> Result<Option<ActivePolicy>> {
    create_schema(conn)?;
    let Some(profile) = read_profile(conn, name)? else {
        return Ok(None);
    };
    let rules = list_active_rules(conn, &profile.name)?;
    Ok(Some(ActivePolicy { profile, rules }))
}

#[derive(Clone, Debug)]
pub(crate) struct ActivePolicy {
    pub(crate) profile: PolicyProfileReport,
    pub(crate) rules: Vec<ActiveRule>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveRule {
    pub(crate) id: String,
    pub(crate) priority: i64,
    pub(crate) mode: PolicyRuleMode,
    pub(crate) anchor: PolicyAnchor,
    pub(crate) predicates: PolicyPredicatesReport,
    pub(crate) preferences: Vec<ActivePreference>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActivePreference {
    pub(crate) effect: PolicyEffect,
    pub(crate) target: PolicyTarget,
    pub(crate) weight: f64,
}

#[derive(Clone, Debug)]
struct PendingPreference {
    effect: PolicyEffect,
    target: PolicyTarget,
    weight: f64,
}

fn read_profile(conn: &Connection, name: &str) -> Result<Option<PolicyProfileReport>> {
    conn.query_row(
        r#"
        SELECT name, mode, strict, default_decision, active, description, updated_at_unix
        FROM router_policy_profiles
        WHERE name = ?1
        "#,
        params![name],
        profile_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn read_active_profile(conn: &Connection) -> Result<Option<PolicyProfileReport>> {
    conn.query_row(
        r#"
        SELECT name, mode, strict, default_decision, active, description, updated_at_unix
        FROM router_policy_profiles
        WHERE active = 1
        ORDER BY updated_at_unix DESC, name ASC
        LIMIT 1
        "#,
        [],
        profile_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn list_profiles(conn: &Connection) -> Result<Vec<PolicyProfileReport>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT name, mode, strict, default_decision, active, description, updated_at_unix
        FROM router_policy_profiles
        ORDER BY active DESC, name ASC
        "#,
    )?;
    let profiles = stmt
        .query_map([], profile_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Error::from)?;
    Ok(profiles)
}

fn list_rules(conn: &Connection, profile: Option<&str>) -> Result<Vec<PolicyRuleReport>> {
    let mut rules = Vec::new();
    if let Some(profile) = profile {
        let mut stmt = conn.prepare(
            r#"
            SELECT id FROM router_policy_rules
            WHERE profile = ?1
            ORDER BY priority DESC, ordinal ASC, id ASC
            "#,
        )?;
        for id in stmt.query_map(params![profile], |row| row.get::<_, String>(0))? {
            if let Some(rule) = read_rule(conn, &id?)? {
                rules.push(rule);
            }
        }
    } else {
        let mut stmt = conn.prepare(
            r#"
            SELECT id FROM router_policy_rules
            ORDER BY profile ASC, priority DESC, ordinal ASC, id ASC
            "#,
        )?;
        for id in stmt.query_map([], |row| row.get::<_, String>(0))? {
            if let Some(rule) = read_rule(conn, &id?)? {
                rules.push(rule);
            }
        }
    }
    Ok(rules)
}

fn list_active_rules(conn: &Connection, profile: &str) -> Result<Vec<ActiveRule>> {
    let mut active = Vec::new();
    for rule in list_rules(conn, Some(profile))? {
        if !rule.enabled {
            continue;
        }
        let preferences = rule
            .preferences
            .iter()
            .map(|preference| ActivePreference {
                effect: preference.effect,
                target: PolicyTarget {
                    kind: preference.target_kind,
                    value: preference.target_value.clone(),
                },
                weight: preference.weight.unwrap_or_else(|| {
                    derived_weight(preference.effect, preference.ordinal as usize)
                }),
            })
            .collect();
        active.push(ActiveRule {
            id: rule.id,
            priority: rule.priority,
            mode: rule.mode,
            anchor: rule.anchor,
            predicates: rule.predicates,
            preferences,
        });
    }
    Ok(active)
}

fn read_rule(conn: &Connection, id: &str) -> Result<Option<PolicyRuleReport>> {
    let Some((id, profile, priority, mode, anchor, ordinal, enabled)) = conn
        .query_row(
            r#"
            SELECT id, profile, priority, mode, anchor, ordinal, enabled
            FROM router_policy_rules
            WHERE id = ?1
            "#,
            params![id],
            |row| {
                let mode_text: String = row.get(3)?;
                let anchor_text: String = row.get(4)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    mode_text,
                    anchor_text,
                    row.get::<_, i64>(5)?,
                    row.get::<_, bool>(6)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    let mode = parse_rule_mode(&mode)?;
    let anchor = parse_anchor(&anchor)?;
    Ok(Some(PolicyRuleReport {
        id: id.clone(),
        profile,
        priority,
        mode,
        anchor,
        ordinal,
        enabled,
        predicates: read_predicates(conn, &id)?,
        preferences: read_preferences(conn, &id)?,
    }))
}

fn read_predicates(conn: &Connection, rule_id: &str) -> Result<PolicyPredicatesReport> {
    let mut stmt = conn.prepare(
        r#"
        SELECT kind, phrase
        FROM router_policy_predicates
        WHERE rule_id = ?1
        ORDER BY kind ASC, ordinal ASC
        "#,
    )?;
    let mut report = PolicyPredicatesReport::default();
    for row in stmt.query_map(params![rule_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (kind, phrase) = row?;
        match kind.as_str() {
            "any_keywords" => report.any_keywords.push(phrase),
            "all_keywords" => report.all_keywords.push(phrase),
            "none_keywords" => report.none_keywords.push(phrase),
            _ => {}
        }
    }
    Ok(report)
}

fn read_preferences(conn: &Connection, rule_id: &str) -> Result<Vec<PolicyPreferenceReport>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT ordinal, effect, target_kind, target_value, weight
        FROM router_policy_preferences
        WHERE rule_id = ?1
        ORDER BY ordinal ASC
        "#,
    )?;
    let mut preferences = Vec::new();
    for row in stmt.query_map(params![rule_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<f64>>(4)?,
        ))
    })? {
        let (ordinal, effect, target_kind, target_value, weight) = row?;
        preferences.push(PolicyPreferenceReport {
            ordinal,
            effect: parse_effect(&effect)?,
            target_kind: parse_target_kind(&target_kind)?,
            target_value,
            weight,
        });
    }
    Ok(preferences)
}

fn build_preferences(options: &PolicySetRuleOptions) -> Result<Vec<PendingPreference>> {
    let mut preferences = Vec::new();
    add_preferences(&mut preferences, PolicyEffect::Prefer, &options.prefer)?;
    add_preferences(&mut preferences, PolicyEffect::Allow, &options.allow)?;
    add_preferences(&mut preferences, PolicyEffect::Suppress, &options.suppress)?;
    add_preferences(&mut preferences, PolicyEffect::Forbid, &options.forbid)?;
    Ok(preferences)
}

fn add_preferences(
    preferences: &mut Vec<PendingPreference>,
    effect: PolicyEffect,
    values: &[String],
) -> Result<()> {
    let offset = preferences.len();
    for (index, target) in parse_targets(values)?.into_iter().enumerate() {
        preferences.push(PendingPreference {
            effect,
            target,
            weight: derived_weight(effect, offset + index),
        });
    }
    Ok(())
}

fn policy_warnings(
    conn: &Connection,
    profile: &str,
    preferences: &[PendingPreference],
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let rule_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM router_policy_rules WHERE profile = ?1",
        params![profile],
        |row| row.get(0),
    )?;
    if rule_count >= 100 {
        warnings.push(format!(
            "profile {profile:?} has more than 100 rules; policy may be hard to audit"
        ));
    }
    if preferences.len() > 10 {
        warnings.push("rule has more than 10 preferences; policy may be hard to audit".to_owned());
    }
    let mut seen = BTreeSet::new();
    for preference in preferences {
        let key = (
            preference.effect.as_str(),
            preference.target.kind.as_str(),
            preference.target.value.as_str(),
        );
        if !seen.insert(key) {
            warnings.push(format!(
                "duplicate preference {}:{}:{}",
                preference.effect.as_str(),
                preference.target.kind.as_str(),
                preference.target.value
            ));
        }
        match preference.target.kind {
            PolicyTargetKind::Skill => {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM skills WHERE name = ?1",
                    params![&preference.target.value],
                    |row| row.get(0),
                )?;
                if count == 0 {
                    warnings.push(format!("unknown skill target {}", preference.target.value));
                }
            }
            PolicyTargetKind::Tag => {
                let pattern = format!("%{}%", preference.target.value);
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM skills WHERE tags_json LIKE ?1",
                    params![pattern],
                    |row| row.get(0),
                )?;
                if count == 0 {
                    warnings.push(format!("unknown tag target {}", preference.target.value));
                }
            }
            PolicyTargetKind::Source | PolicyTargetKind::HasSkillSpec => {}
        }
    }
    Ok(warnings)
}

fn require_profile(conn: &Connection, profile: &str) -> Result<PolicyProfileReport> {
    if let Some(profile) = read_profile(conn, profile)? {
        return Ok(profile);
    }
    Err(Error::InvalidInput {
        message: format!("router policy profile {profile:?} does not exist"),
    })
}

fn insert_predicates(
    conn: &Connection,
    rule_id: &str,
    kind: &str,
    phrases: &[String],
) -> Result<()> {
    for (ordinal, phrase) in phrases.iter().enumerate() {
        if phrase.trim().is_empty() {
            continue;
        }
        conn.execute(
            r#"
            INSERT INTO router_policy_predicates (rule_id, kind, phrase, ordinal)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![rule_id, kind, phrase.trim(), ordinal as i64],
        )?;
    }
    Ok(())
}

fn next_rule_ordinal(conn: &Connection, profile: &str, id: &str) -> Result<i64> {
    if let Some(ordinal) = conn
        .query_row(
            "SELECT ordinal FROM router_policy_rules WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(ordinal);
    }
    let next = conn.query_row(
        "SELECT COALESCE(MAX(ordinal), -1) + 1 FROM router_policy_rules WHERE profile = ?1",
        params![profile],
        |row| row.get(0),
    )?;
    Ok(next)
}

fn profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PolicyProfileReport> {
    let mode_text: String = row.get(1)?;
    let mode = PolicyProfileMode::parse(&mode_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            format!("unknown policy profile mode {mode_text:?}").into(),
        )
    })?;
    Ok(PolicyProfileReport {
        name: row.get(0)?,
        mode,
        strict: row.get(2)?,
        default_decision: row.get(3)?,
        active: row.get(4)?,
        description: row.get(5)?,
        updated_at_unix: row.get(6)?,
    })
}

fn parse_rule_mode(value: &str) -> Result<PolicyRuleMode> {
    PolicyRuleMode::parse(value).ok_or_else(|| Error::InvalidInput {
        message: format!("unknown router policy rule mode {value:?}"),
    })
}

fn parse_anchor(value: &str) -> Result<PolicyAnchor> {
    PolicyAnchor::parse(value).ok_or_else(|| Error::InvalidInput {
        message: format!("unknown router policy anchor {value:?}"),
    })
}

fn parse_effect(value: &str) -> Result<PolicyEffect> {
    PolicyEffect::parse(value).ok_or_else(|| Error::InvalidInput {
        message: format!("unknown router policy effect {value:?}"),
    })
}

fn parse_target_kind(value: &str) -> Result<PolicyTargetKind> {
    PolicyTargetKind::parse(value).ok_or_else(|| Error::InvalidInput {
        message: format!("unknown router policy target kind {value:?}"),
    })
}

fn validate_name(label: &str, value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if valid {
        return Ok(());
    }
    Err(Error::InvalidInput {
        message: format!("invalid router policy {label} name {value:?}"),
    })
}

fn default_decision(mode: PolicyProfileMode) -> Option<&'static str> {
    match mode {
        PolicyProfileMode::Route => None,
        PolicyProfileMode::SoftPassthrough | PolicyProfileMode::NativePassthrough => Some("bypass"),
    }
}

fn derived_weight(effect: PolicyEffect, ordinal: usize) -> f64 {
    let weight = (100.0 - (ordinal as f64 * 20.0)).max(1.0);
    match effect {
        PolicyEffect::Prefer | PolicyEffect::Allow | PolicyEffect::Forbid => weight,
        PolicyEffect::Suppress => weight,
    }
}

fn record_event(
    conn: &Connection,
    action: &str,
    profile: Option<&str>,
    summary: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO router_policy_events (at_unix, actor, action, profile, summary)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![now_unix(), "cli", action, profile, summary],
    )?;
    Ok(())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
