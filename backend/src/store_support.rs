use crate::{
    actor::AuthenticatedActor,
    domain::{AccountCode, AdjustmentLine, DomainError, FinancialSnapshot},
    store::{StoreError, WorkspaceImportRequest},
};
use rust_decimal::Decimal;
use std::collections::BTreeSet;

pub(crate) fn validate_import_request(request: &WorkspaceImportRequest) -> Result<(), StoreError> {
    let required = [
        ("entity_name", request.entity_name.as_str()),
        ("registration_number", request.registration_number.as_str()),
        ("jurisdiction", request.jurisdiction.as_str()),
        ("entity_type", request.entity_type.as_str()),
        ("owner_name", request.owner_name.as_str()),
        ("firm_name", request.firm_name.as_str()),
        ("preparer_name", request.preparer_name.as_str()),
        ("reviewer_name", request.reviewer_name.as_str()),
        ("client_signer_name", request.client_signer_name.as_str()),
        ("branch_label", request.branch_label.as_str()),
        ("source_label", request.source_label.as_str()),
    ];

    for (field, value) in required {
        if value.trim().is_empty() {
            return Err(StoreError::InvalidImport(format!("{field} is required")));
        }
    }

    if request.period_start > request.period_end {
        return Err(StoreError::InvalidImport(
            "period_start must be before period_end".to_string(),
        ));
    }

    if request.trial_balance.is_empty() {
        return Err(StoreError::InvalidImport(
            "trial_balance must include at least one account".to_string(),
        ));
    }

    let mut account_codes = BTreeSet::new();
    for line in &request.trial_balance {
        let required_line = [
            ("account_code", line.account_code.as_str()),
            ("account_name", line.account_name.as_str()),
            ("fs_line", line.fs_line.as_str()),
            ("assertion", line.assertion.as_str()),
        ];
        for (field, value) in required_line {
            if value.trim().is_empty() {
                return Err(StoreError::InvalidImport(format!(
                    "{field} is required for every trial balance line"
                )));
            }
        }

        AccountCode::parse(&line.account_code).map_err(|error| {
            StoreError::InvalidImport(format!(
                "invalid account_code {}: {error}",
                line.account_code
            ))
        })?;

        if !account_codes.insert(line.account_code.trim().to_string()) {
            return Err(StoreError::InvalidImport(format!(
                "duplicate account code {}",
                line.account_code.trim()
            )));
        }
    }

    let total = request
        .trial_balance
        .iter()
        .map(|line| line.amount)
        .sum::<Decimal>();
    if !total.is_zero() {
        return Err(StoreError::InvalidImport(
            "trial_balance must balance to zero".to_string(),
        ));
    }

    Ok(())
}

pub(crate) fn email_or_default(email: &str, name: &str, domain: &str) -> String {
    if email.trim().is_empty() {
        user_email(name, domain)
    } else {
        email.trim().to_ascii_lowercase()
    }
}

pub(crate) fn actor_id_for_email(actor: &AuthenticatedActor, email: &str) -> Option<String> {
    actor
        .email
        .eq_ignore_ascii_case(email)
        .then(|| actor.auth_user_id.clone())
}

pub(crate) fn validate_adjustment_accounts(
    snapshot: &FinancialSnapshot,
    lines: &[AdjustmentLine],
) -> Result<(), DomainError> {
    let account_codes = snapshot
        .trial_balance
        .iter()
        .map(|line| line.account_code.as_str())
        .collect::<BTreeSet<_>>();
    let mapped_codes = snapshot
        .mappings
        .iter()
        .map(|mapping| mapping.account_code.as_str())
        .collect::<BTreeSet<_>>();

    for line in lines {
        if !account_codes.contains(line.account_code.as_str()) {
            return Err(DomainError::UnknownAdjustmentAccount(
                line.account_code.clone(),
            ));
        }
        if !mapped_codes.contains(line.account_code.as_str()) {
            return Err(DomainError::UnmappedAdjustmentAccount(
                line.account_code.clone(),
            ));
        }
    }

    Ok(())
}

fn user_email(name: &str, domain: &str) -> String {
    let local = name
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(".");

    format!("{}@{}", local, domain)
}
