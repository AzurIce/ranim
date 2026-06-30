---
name: edit-pr
description: Edit PR description with structured format
allowed-tools: Bash(gh:*), Bash(git:*), Read, Grep
argument-hint: [pr-number]
---

Edit the PR description with a structured format following ranim's conventions (reference: PR #77, PR #64).

## Task

1. **Identify the PR**: If no PR number is provided, get the current branch's PR with `gh pr list --head $(git branch --show-current)`.

2. **Analyze the changes**:
   - Run `git diff main...HEAD --stat` to understand the scope
   - Run `git log main..HEAD --oneline` to see commits
   - Identify what files/modules were changed

3. **Understand the context**: Read key changed files to understand what the PR is doing. Look for:
   - New features or functionality
   - Bug fixes
   - Breaking changes (API changes, removed fields, changed behavior)
   - Refactorings

4. **Structure the PR description** following this format:

```
Closes: #<issue-number>

- **feat**: New features or capabilities
- **fix**: Bug fixes
- **refactor**: Code improvements without behavior change
- **docs**: Documentation updates
- **perf**: Performance improvements
- **test**: Test additions or changes

### Breaking Changes

- **API/Field name**: Description of the breaking change and migration path

---

## Component/Feature 1

Detailed description of the first major change.

Use code blocks, mermaid diagrams, or examples as needed.

## Component/Feature 2

...
```

5. **Use visualizations when helpful**:
   - Mermaid flowcharts for workflows, pipelines, or state machines
   - Code snippets for API changes
   - Before/after comparisons for refactorings

6. **Update the PR** with `gh pr edit <number> --body "$(cat <<'EOF'\n...\nEOF\n)"`

## Guidelines

- **Top section**: Concise bullet list categorized by change type (feat/fix/refactor/etc)
- **Breaking changes**: Always a separate section if any exist
- **Detail sections**: Group related changes logically by component/feature
- **Be specific**: Focus on "what changed" and "why it matters"
- **Visual aids**: Use mermaid diagrams for complex structures (graphs, flows, architectures)
- **Examples over prose**: Show examples instead of long explanations
- **Omit noise**: Skip implementation details unless they're the point

Return the PR URL at the end.
