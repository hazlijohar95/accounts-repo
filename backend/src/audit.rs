use crate::{
    actor::AuthenticatedActor,
    domain::{AuditEvent, AuditEventType},
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(crate) fn push_audit_event(
    events: &mut Vec<AuditEvent>,
    legal_entity_id: Uuid,
    actor: &AuthenticatedActor,
    event_type: AuditEventType,
    message: String,
    related_commit_id: Option<Uuid>,
) {
    let sequence_number = events.len() as u64 + 1;
    let previous_hash = events.last().map(|event| event.event_hash.clone());
    let occurred_at = Utc::now();
    let event_hash = audit_hash(
        legal_entity_id,
        sequence_number,
        previous_hash.as_deref(),
        actor,
        &event_type,
        &message,
        occurred_at.to_rfc3339().as_str(),
        related_commit_id,
    );

    events.push(AuditEvent {
        id: Uuid::new_v4(),
        legal_entity_id,
        sequence_number,
        actor_user_id: Some(actor.auth_user_id.clone()),
        actor_name: actor.display_name.clone(),
        actor_email: actor.email.clone(),
        event_type,
        message,
        occurred_at,
        related_commit_id,
        previous_hash,
        event_hash,
    });
}

fn audit_hash(
    legal_entity_id: Uuid,
    sequence_number: u64,
    previous_hash: Option<&str>,
    actor: &AuthenticatedActor,
    event_type: &AuditEventType,
    message: &str,
    occurred_at: &str,
    related_commit_id: Option<Uuid>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(legal_entity_id.as_bytes());
    hasher.update(sequence_number.to_be_bytes());
    hasher.update(previous_hash.unwrap_or_default().as_bytes());
    hasher.update(actor.auth_user_id.as_bytes());
    hasher.update(actor.email.as_bytes());
    hasher.update(format!("{:?}", event_type).as_bytes());
    hasher.update(message.as_bytes());
    hasher.update(occurred_at.as_bytes());
    if let Some(commit_id) = related_commit_id {
        hasher.update(commit_id.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
