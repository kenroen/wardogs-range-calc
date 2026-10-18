---
name: lessons-learned
description: >-
  Self-learning knowledge base of past mistakes, root-caused bugs, user corrections, and shell/domain pitfalls in wardogs-range-calc. Activate this skill BEFORE debugging, modifying ballistics/elevation math, or running complex PowerShell/git commands, and ALWAYS update it whenever a mistake, test failure, or user correction occurs.
---

# Self-Learning Lessons & Known Pitfalls

This skill maintains a living log of mistakes, tricky edge cases, and user corrections encountered while developing `wardogs-range-calc` so the agent never repeats the same mistake twice.

## When to Read

1. **Before debugging or changing ballistics/elevation/coordinate logic**: Read [`references/known-pitfalls.md`](./references/known-pitfalls.md) first to check if the issue or constraint has already been documented.
2. **Before running multi-line PowerShell or Git commands**: Check the shell pitfalls section in [`references/known-pitfalls.md`](./references/known-pitfalls.md).

## When to Update (Self-Learning Loop)

Whenever **any** of the following happens during a session, immediately append or update an entry in [`references/known-pitfalls.md`](./references/known-pitfalls.md) before finishing the task:
- A command fails due to shell syntax, escaping, or OS differences (Windows / `pwsh`).
- `cargo clippy --all-targets -- -D warnings` or `cargo test --locked` catches a non-obvious error.
- The user points out a discrepancy, bug, or preference correction.
- A subtle mathematical, coordinate, or UI behavior is discovered while investigating a bug.

### Entry Template for `references/known-pitfalls.md`

Keep each entry concise and actionable:

```markdown
### <Short Title>
- **Symptom / Mistake**: What went wrong or what failed.
- **Root Cause**: Why it happened.
- **Rule / Fix**: The exact pattern or command to use instead.
```
