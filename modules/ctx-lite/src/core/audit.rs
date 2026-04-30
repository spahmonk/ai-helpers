use crate::core::redaction::RedactionPipeline;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditEventKind {
    FileRead,
    ShellDecision,
    DoctorCheck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditOutcome {
    Allowed,
    Denied,
    Passed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub kind: AuditEventKind,
    pub subject: String,
    pub outcome: AuditOutcome,
    pub bytes: Option<u64>,
    pub detail: Option<String>,
    pub redactions_applied: usize,
}

impl AuditEvent {
    pub fn file_read(path: impl AsRef<str>, bytes: u64, outcome: AuditOutcome) -> Self {
        let pipeline = RedactionPipeline::default();
        let redacted = pipeline.redact(path.as_ref());

        Self {
            kind: AuditEventKind::FileRead,
            subject: redacted.text,
            outcome,
            bytes: Some(bytes),
            detail: None,
            redactions_applied: redacted.redactions_applied,
        }
    }

    pub fn shell_decision(command: impl AsRef<str>, outcome: AuditOutcome) -> Self {
        let pipeline = RedactionPipeline::default();
        let redacted = pipeline.redact(command.as_ref());

        Self {
            kind: AuditEventKind::ShellDecision,
            subject: redacted.text,
            outcome,
            bytes: None,
            detail: None,
            redactions_applied: redacted.redactions_applied,
        }
    }

    pub fn doctor_check(
        name: impl AsRef<str>,
        passed: bool,
        detail: Option<impl AsRef<str>>,
    ) -> Self {
        let pipeline = RedactionPipeline::default();
        let redacted_name = pipeline.redact(name.as_ref());
        let redacted_detail = detail.map(|detail| pipeline.redact(detail.as_ref()));

        Self {
            kind: AuditEventKind::DoctorCheck,
            subject: redacted_name.text,
            outcome: if passed {
                AuditOutcome::Passed
            } else {
                AuditOutcome::Failed
            },
            bytes: None,
            detail: redacted_detail.as_ref().map(|detail| detail.text.clone()),
            redactions_applied: redacted_name.redactions_applied
                + redacted_detail
                    .as_ref()
                    .map(|detail| detail.redactions_applied)
                    .unwrap_or(0),
        }
    }

    pub fn redacted(&self, pipeline: &RedactionPipeline) -> Self {
        let subject = pipeline.redact(&self.subject);
        let detail = self.detail.as_deref().map(|detail| pipeline.redact(detail));

        Self {
            kind: self.kind,
            subject: subject.text,
            outcome: self.outcome,
            bytes: self.bytes,
            detail: detail.as_ref().map(|detail| detail.text.clone()),
            redactions_applied: self.redactions_applied
                + subject.redactions_applied
                + detail
                    .as_ref()
                    .map(|detail| detail.redactions_applied)
                    .unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuditEvent, AuditEventKind, AuditOutcome};
    use crate::core::redaction::RedactionPipeline;

    #[test]
    fn shell_events_are_redacted_before_persistence() {
        let event = AuditEvent::shell_decision("echo token=abc123", AuditOutcome::Denied);

        assert_eq!(event.kind, AuditEventKind::ShellDecision);
        assert!(!event.subject.contains("abc123"));

        let redacted = event.redacted(&RedactionPipeline::default());
        assert!(redacted.subject.contains("[REDACTED]"));
    }

    #[test]
    fn shell_events_fully_redact_slash_containing_secrets() {
        let event = AuditEvent::shell_decision(
            "curl -H 'Authorization: Bearer abc/def' https://example.test/api?access_token=ghi/jkl",
            AuditOutcome::Denied,
        );

        assert_eq!(
            event.subject,
            "curl -H 'Authorization: Bearer [REDACTED]' https://example.test/api?access_token=[REDACTED]"
        );
        assert!(event.redactions_applied >= 2);
    }
}
