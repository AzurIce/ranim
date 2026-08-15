# AGENTS — PR Authoring Guidelines

This file describes how to write and update pull request descriptions in this
repository. It applies to both human contributors and AI coding agents.

## PR Description Format

When creating or editing a PR, follow this structure:

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

### Top section

- Concise bullet list categorized by change type (`feat` / `fix` / `refactor` /
  `docs` / `perf` / `test`).
- Include `Closes: #<issue-number>` only when the PR closes an issue.

### Breaking changes

- Always a separate section if any exist.
- For each breaking change, name the API or field and describe the migration path.

### Detail sections

- Group related changes logically by component or feature.
- Focus on "what changed" and "why it matters".
- Use examples instead of long prose explanations.
- Use visuals when helpful:
  - Mermaid flowcharts for workflows, pipelines, or state machines.
  - Code snippets for API changes.
  - Before/after comparisons for refactorings.
- Omit implementation noise unless it is the point.

---

# AGENTS — PR 编写规范

本文件描述本仓库中 pull request 描述的编写与更新规则，适用于人类贡献者和 AI
编码代理。

## PR 描述格式

创建或编辑 PR 时，请遵循以下结构：

```
Closes: #<issue-number>

- **feat**: 新增功能或能力
- **fix**: 错误修复
- **refactor**: 不改变行为的代码改进
- **docs**: 文档更新
- **perf**: 性能改进
- **test**: 测试新增或修改

### Breaking Changes

- **API/字段名**: 破坏性变更的描述以及迁移路径

---

## 组件/功能 1

第一个主要变更的详细描述。

根据需要可使用代码块、mermaid 图或示例。

## 组件/功能 2

...
```

### 顶部小节

- 按变更类型（`feat` / `fix` / `refactor` / `docs` / `perf` / `test`）分类的简洁列表。
- 仅当 PR 关闭某个 issue 时才写 `Closes: #<issue-number>`。

### 破坏性变更

- 只要有破坏性变更，就必须单独成节。
- 对每项破坏性变更，指出 API 或字段名，并说明迁移路径。

### 详情小节

- 按组件或功能将相关变更进行逻辑分组。
- 聚焦于“改了什么”和“为什么重要”。
- 多用示例，少用冗长的文字说明。
- 合适时使用可视化：
  - 工作流、流水线或状态机使用 mermaid 流程图。
  - API 变更使用代码片段。
  - 重构使用 before/after 对比。
- 省略实现噪音，除非其本身就是重点。
