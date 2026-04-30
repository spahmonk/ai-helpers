use ctx_lite::core::audit::{AuditEvent, AuditEventKind, AuditOutcome};
use ctx_lite::core::redaction::RedactionPipeline;
use ctx_lite::core::stats::{OperationKind, StatsSnapshot};

#[test]
fn redaction_masks_secrets_before_persistence() {
    let pipeline = RedactionPipeline::default();

    let redacted = pipeline.redact_text("token=abc123 password: super-secret");

    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("abc123"));
    assert!(!redacted.contains("super-secret"));
}

#[test]
fn audit_events_store_redacted_payloads_for_sensitive_operations() {
    let pipeline = RedactionPipeline::default();
    let event = AuditEvent::shell_decision(
        "echo token=abc123 password: super-secret",
        AuditOutcome::Denied,
    );
    let sanitized = event.redacted(&pipeline);

    assert_eq!(sanitized.kind, AuditEventKind::ShellDecision);
    assert_eq!(sanitized.outcome, AuditOutcome::Denied);
    assert!(sanitized.subject.contains("[REDACTED]"));
    assert!(!sanitized.subject.contains("abc123"));
    assert!(!sanitized.subject.contains("super-secret"));
}

#[test]
fn audit_events_for_file_reads_and_doctor_checks_stay_structured() {
    let file_read = AuditEvent::file_read("/repo/src/lib.rs", 42, AuditOutcome::Allowed);
    let doctor = AuditEvent::doctor_check("storage", true, Some("checked /repo/.ctx/token=abc123"));

    assert_eq!(file_read.kind, AuditEventKind::FileRead);
    assert_eq!(file_read.bytes, Some(42));
    assert_eq!(doctor.kind, AuditEventKind::DoctorCheck);
    assert_eq!(doctor.outcome, AuditOutcome::Passed);
    assert!(doctor.detail.as_deref().unwrap().contains("[REDACTED]"));
}

#[test]
fn file_read_audit_events_redact_query_values_without_breaking_paths() {
    let file_read = AuditEvent::file_read(
        "https://example.test/download?access_token=abc123&path=/repo/src/lib.rs",
        42,
        AuditOutcome::Allowed,
    );

    assert_eq!(file_read.kind, AuditEventKind::FileRead);
    assert_eq!(
        file_read.subject,
        "https://example.test/download?access_token=[REDACTED]&path=/repo/src/lib.rs"
    );
    assert_eq!(file_read.redactions_applied, 1);
    assert!(!file_read.subject.contains("abc123"));
}

#[test]
fn stats_snapshots_only_include_safe_aggregates() {
    let mut stats = StatsSnapshot::default();
    stats.record(OperationKind::FileRead, 120, 80, 2);
    stats.record(OperationKind::ShellDecision, 0, 0, 1);

    let file_read = stats
        .operation(OperationKind::FileRead)
        .expect("file read stats should exist");
    let shell = stats
        .operation(OperationKind::ShellDecision)
        .expect("shell decision stats should exist");

    assert_eq!(file_read.count, 1);
    assert_eq!(file_read.input_bytes, 120);
    assert_eq!(file_read.output_bytes, 80);
    assert_eq!(file_read.saved_bytes, 40);
    assert_eq!(file_read.redactions_applied, 2);
    assert_eq!(shell.redactions_applied, 1);
}
