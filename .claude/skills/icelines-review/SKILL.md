---
name: icelines-review
description: "Review IceLines plans, pulses, specs, or code through .roles and write actionable findings into the active wave panels directory."
tags: [icelines, review, roles, panels, findings]
---

# icelines-review

Use this skill to review a plan, pulse, spec, scenario, or implemented slice.

## Commands

```text
/icelines-review pulse 02
/icelines-review plan Jack-Adams-Web
/icelines-review spec design/specs/surface-parity.md
/icelines-review code icelines-web/src/handlers/dashboard.rs
```

## Roles

Roles live in `.roles/`.

Use a small panel matched to the work:

| Work | Suggested Panel |
|---|---|
| Architecture / crate boundaries | `keel`, `forge`, `edge` |
| Product surface parity | `wire`, `bench`, `scout` |
| Tests / fixtures / gates | `tape`, `glass`, `forge` |
| Visual quality | `crest`, `broadcast`, `glass` |
| Web/TUI UX | `bench`, `crest`, `wire`, `edge` |
| Release / docs truth | `tape`, `broadcast`, `wire` |

## Output

Write findings under:

```text
design/waves/{active}/panels/{review-name}/R{N}-{role}.md
```

Finding format:

```markdown
# R{N} Review - {role}

## Findings

### F-01 - BLOCK: {title}
File: {path}
Finding: {specific issue}
Consequence: {what breaks}
Fix: {concrete change}
```

Use `BLOCK`, `WARN`, and `NOTE`. Prefer no finding over speculative noise.
