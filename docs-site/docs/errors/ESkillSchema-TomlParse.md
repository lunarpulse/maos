---
title: "ESkillSchema::TomlParse"
sidebar_position: 0
description: "Error reference for ESkillSchema::TomlParse"
slug: "/errors/ESkillSchema::TomlParse"
---

# ESkillSchema::TomlParse

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::TomlParse` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

TOML frontmatter failed to parse (malformed syntax, missing required field, wrong value type)

## Rust Path

`ESkillSchema::TomlParse`

## Cause

The TOML content within the frontmatter fences failed to parse. This is caused by malformed TOML syntax (unclosed quotes, invalid escapes), a missing required field, or a field with the wrong value type (e.g. string where integer expected).

## Resolution

Validate the TOML frontmatter with a TOML linter (e.g. `taplo check`). Fix any syntax errors, add missing required fields, and ensure value types match the schema. Common issues: unquoted strings, missing commas in arrays, bare keys with special characters.

## Source Location

```rust
// Emitted from: ESkillSchema::TomlParse
// Crate: maos-skill
Err(ESkillSchema::TomlParse)
```
