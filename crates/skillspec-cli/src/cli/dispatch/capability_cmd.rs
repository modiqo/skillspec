use crate::cli::args::CapabilityCommand;
use skillspec::{capability, error::Result, report};

pub(super) fn run(command: CapabilityCommand) -> Result<()> {
    match command {
        CapabilityCommand::Store => {
            report::json(&capability::store()?)?;
        }
        CapabilityCommand::Add {
            id,
            domain,
            kind,
            command,
            adapter,
            script,
            provides,
            alias,
            priority,
            preferred_for,
            avoid_for,
            ties,
            auth_env,
            external_service,
            may_cost_money,
            evidence_command,
            suggested_skill_id,
        } => {
            let report = capability::add(capability::AddOptions {
                id,
                domain,
                kind,
                command,
                adapter,
                script,
                provides,
                aliases: alias,
                priority,
                preferred_for,
                avoid_for,
                ties,
                auth_env,
                external_service,
                may_cost_money,
                evidence_command,
                suggested_skill_id,
            })?;
            report::json(&report)?;
        }
        CapabilityCommand::Update {
            id,
            domain,
            kind,
            command,
            clear_command,
            adapter,
            clear_adapter,
            script,
            clear_script,
            add_provides,
            remove_provides,
            add_alias,
            remove_alias,
            priority,
            clear_priority,
            add_preferred_for,
            remove_preferred_for,
            add_avoid_for,
            remove_avoid_for,
            add_tie,
            remove_tie,
            add_auth_env,
            remove_auth_env,
            external_service,
            may_cost_money,
            add_evidence_command,
            remove_evidence_command,
            suggested_skill_id,
            clear_suggested_skill_id,
            mark_unverified,
            mark_failed,
        } => {
            let verification_status = if mark_failed {
                Some(capability::VerificationStatus::Failed)
            } else if mark_unverified {
                Some(capability::VerificationStatus::Unverified)
            } else {
                None
            };
            let report = capability::update(capability::UpdateOptions {
                id,
                domain,
                kind,
                command,
                clear_command,
                adapter,
                clear_adapter,
                script,
                clear_script,
                add_provides,
                remove_provides,
                add_alias,
                remove_alias,
                priority,
                clear_priority,
                add_preferred_for,
                remove_preferred_for,
                add_avoid_for,
                remove_avoid_for,
                add_ties: add_tie,
                remove_tie,
                add_auth_env,
                remove_auth_env,
                external_service,
                may_cost_money,
                add_evidence_command,
                remove_evidence_command,
                suggested_skill_id,
                clear_suggested_skill_id,
                verification_status,
            })?;
            report::json(&report)?;
        }
        CapabilityCommand::List { domain } => {
            report::json(&capability::list(domain.as_deref())?)?;
        }
        CapabilityCommand::Search {
            capability: capability_id,
            domain,
            explain: _,
            json: _,
            local_only,
            preferred_seed,
        } => {
            let report = capability::search(capability::SearchOptions {
                capability: capability_id,
                domain,
                local_only,
                preferred_seed,
            })?;
            report::json(&report)?;
        }
        CapabilityCommand::Inspect {
            id,
            domain,
            json: _,
        } => {
            report::json(&capability::inspect(&id, domain.as_deref())?)?;
        }
        CapabilityCommand::Verify {
            id,
            domain,
            json: _,
        } => {
            report::json(&capability::verify(&id, domain.as_deref())?)?;
        }
        CapabilityCommand::Prefer {
            id,
            domain,
            for_capability,
            priority,
        } => {
            let report = capability::prefer(capability::PreferOptions {
                id,
                domain,
                for_capability,
                priority,
            })?;
            report::json(&report)?;
        }
        CapabilityCommand::Remove { id, domain } => {
            report::json(&capability::remove(&id, domain.as_deref())?)?;
        }
        CapabilityCommand::Scan => {
            report::json(&capability::scan()?)?;
        }
    }

    Ok(())
}
