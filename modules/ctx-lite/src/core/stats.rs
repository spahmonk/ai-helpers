use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    FileRead,
    ShellDecision,
    DoctorCheck,
}

impl fmt::Debug for OperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl OperationKind {
    fn as_str(self) -> &'static str {
        match self {
            OperationKind::FileRead => "file_read",
            OperationKind::ShellDecision => "shell_decision",
            OperationKind::DoctorCheck => "doctor_check",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OperationStats {
    pub count: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub saved_bytes: u64,
    pub redactions_applied: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatsSnapshot {
    operations: BTreeMap<OperationKind, OperationStats>,
}

impl StatsSnapshot {
    pub fn record(
        &mut self,
        kind: OperationKind,
        input_bytes: u64,
        output_bytes: u64,
        redactions_applied: u64,
    ) {
        let entry = self.operations.entry(kind).or_default();
        entry.count += 1;
        entry.input_bytes += input_bytes;
        entry.output_bytes += output_bytes;
        entry.saved_bytes += input_bytes.saturating_sub(output_bytes);
        entry.redactions_applied += redactions_applied;
    }

    pub fn operation(&self, kind: OperationKind) -> Option<&OperationStats> {
        self.operations.get(&kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationKind, StatsSnapshot};

    #[test]
    fn records_safe_aggregates_only() {
        let mut stats = StatsSnapshot::default();
        stats.record(OperationKind::FileRead, 100, 80, 1);

        let file_read = stats
            .operation(OperationKind::FileRead)
            .expect("file read stats should exist");

        assert_eq!(file_read.count, 1);
        assert_eq!(file_read.saved_bytes, 20);
    }
}
