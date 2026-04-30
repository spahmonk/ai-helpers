# Implementation orchestration

## Workflow

- The orchestrator owns scope, sequencing, and review checkpoints.
- Lower-cost agents handle discovery, file reading, and repetitive verification.
- Each task follows: spec review → failing test → minimal code → format/test → review.
- No feature work lands without tests and a final verification pass.

## Review chain

1. Identify the smallest deliverable.
2. Write the failing test first.
3. Implement the minimum code.
4. Run format and tests.
5. Escalate for review before merge.

## Worktree / branch policy

- One task owns one worktree and one topic branch.
- Do not edit or reuse another task's worktree.
- Keep changes scoped to the task branch until review is complete.
- Avoid history rewrites unless explicitly required for release cleanup.
