---
title: RFC Template
sidebar_position: 2
description: Template for feature and process proposals.
---

# RFC Process

RFCs (Requests for Comments) are the mechanism for proposing features,
process changes, and community policies.

> **Template:** [`RFC_TEMPLATE.md`](https://github.com/lunarpulse/maos/blob/main/RFC_TEMPLATE.md)

## When to Write an RFC

- New user-facing features
- Changes to community process or governance
- Cross-cutting technical proposals that span multiple ADRs

## RFCs vs ADRs

- **ADRs** (`docs/adr/`) record architecture decisions that constrain
  the system design. They are binding once accepted.
- **RFCs** are broader proposals that benefit from structured review
  before implementation.

An RFC may result in one or more ADRs if it involves architectural decisions.

## Process

1. Copy the template to `rfcs/RFC-NNN-short-title.md`
2. Fill in all sections; set Status to **Draft**
3. Open a pull request titled `RFC-NNN: Short Title`
4. Collect feedback and iterate
5. Update Status to **Accepted** on consensus
