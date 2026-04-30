#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    Ok,
    WarningThreshold, // 80% used
    Exceeded,
}

pub struct ContextBudget {
    max_tokens: usize,
    consumed_tokens: usize,
    warning_threshold: f32, // 0.8
}

impl ContextBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            consumed_tokens: 0,
            warning_threshold: 0.8,
        }
    }

    pub fn consume(&mut self, tokens: usize) -> BudgetStatus {
        self.consumed_tokens += tokens;
        self.check_status()
    }

    fn check_status(&self) -> BudgetStatus {
        let percent = self.percentage_used();
        if percent > 1.0 {
            BudgetStatus::Exceeded
        } else if percent > self.warning_threshold && percent < 1.0 {
            BudgetStatus::WarningThreshold
        } else {
            BudgetStatus::Ok
        }
    }

    pub fn percentage_used(&self) -> f32 {
        if self.max_tokens == 0 {
            0.0
        } else {
            self.consumed_tokens as f32 / self.max_tokens as f32
        }
    }

    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.consumed_tokens)
    }

    pub fn used(&self) -> usize {
        self.consumed_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetStatus, ContextBudget};

    #[test]
    fn test_budget_creation() {
        let budget = ContextBudget::new(1000);
        assert_eq!(budget.max_tokens, 1000);
        assert_eq!(budget.consumed_tokens, 0);
    }

    #[test]
    fn test_budget_consumption() {
        let mut budget = ContextBudget::new(1000);
        let status = budget.consume(100);
        assert_eq!(budget.used(), 100);
        assert_eq!(status, BudgetStatus::Ok);
    }

    #[test]
    fn test_budget_percentage() {
        let mut budget = ContextBudget::new(1000);
        budget.consume(250);
        assert_eq!(budget.percentage_used(), 0.25);

        budget.consume(250);
        assert_eq!(budget.percentage_used(), 0.5);

        budget.consume(500);
        assert_eq!(budget.percentage_used(), 1.0);
    }

    #[test]
    fn test_warning_threshold() {
        let mut budget = ContextBudget::new(1000);
        budget.consume(700); // 70%
        assert_eq!(budget.check_status(), BudgetStatus::Ok);

        budget.consume(100); // 80% - should be Ok with > not >=
        assert_eq!(budget.check_status(), BudgetStatus::Ok);

        budget.consume(1); // 80.1% - now should trigger WarningThreshold
        assert_eq!(budget.check_status(), BudgetStatus::WarningThreshold);
        
        budget.consume(50); // 85%
        assert_eq!(budget.check_status(), BudgetStatus::WarningThreshold);
    }

    #[test]
    fn test_exceeded_detection() {
        let mut budget = ContextBudget::new(1000);
        budget.consume(1000);
        assert_eq!(budget.check_status(), BudgetStatus::Ok); // 100% is still Ok

        budget.consume(1);
        assert_eq!(budget.check_status(), BudgetStatus::Exceeded); // >100% is exceeded
    }

    #[test]
    fn test_remaining_calculation() {
        let mut budget = ContextBudget::new(1000);
        assert_eq!(budget.remaining(), 1000);

        budget.consume(300);
        assert_eq!(budget.remaining(), 700);

        budget.consume(700);
        assert_eq!(budget.remaining(), 0);

        budget.consume(100); // Overflow
        assert_eq!(budget.remaining(), 0); // saturating_sub keeps it at 0
    }

    #[test]
    fn test_percentage_with_zero_budget() {
        let budget = ContextBudget::new(0);
        assert_eq!(budget.percentage_used(), 0.0);
    }
}
