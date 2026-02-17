# Contributing to Ranim

Thank you for your interest in contributing to Ranim! This document outlines the process for contributing to help keep development organized and review manageable.

## Issue First

**All non-trivial changes require an issue before a PR.**

Ranim is still WIP — the architecture and APIs are evolving. To avoid wasted effort and ensure changes fit the project's direction, please follow this workflow:

1. **Open an issue** describing what you want to do (bug fix, new feature, refactor, etc.)
2. **Discuss the approach** in the issue — wait for agreement before writing code
3. **Submit a PR** that references the issue

This applies to new features, API changes, and significant refactors. For obvious typo fixes or small documentation improvements, you can submit a PR directly.

### Why?

- Prevents you from spending time on work that may not be accepted
- Gives maintainers a chance to suggest a better approach or point out conflicts with ongoing work
- Keeps the codebase coherent as the project evolves

## Pull Requests

- **One PR, one concern** — keep PRs small and focused on a single change
- **Reference the related issue** (e.g., `Closes #123`)
- **Describe what changed and why** in the PR description
- Make sure CI passes before requesting review

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Follow existing patterns in the codebase

## Questions?

If you're unsure whether a change is welcome or how to approach it, feel free to open an issue to discuss.
