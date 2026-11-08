use crate::model::{Incident, Severity};

pub fn render(incident: &Incident) -> String {
    let body = sanitize(format!(
        "Incident {id}\n\
         Service: {service}\n\
         Severity: {severity:?}\n\
         Opened: {opened}\n\
         Owner: {owner}\n\
         Summary: {summary}\n\
         Impact: {impact}\n\
         Next update: {next}\n",
        id = incident.id,
        service = incident.service,
        severity = incident.severity,
        opened = incident.opened_at.to_rfc3339(),
        owner = incident.owner.as_deref().unwrap_or("unassigned"),
        summary = incident.summary.trim(),
        impact = incident.impact.as_deref().unwrap_or("unknown"),
        next = incident.next_update.map(|t| t.to_rfc3339()).unwrap_or_default(),
    ));
    body
}

pub fn is_urgent(incident: &Incident) -> bool {
    matches!(incident.severity, Severity::Critical | Severity::High)
}
