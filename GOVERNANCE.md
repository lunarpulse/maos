# Governance

This document describes the governance model for the MAOS project.

## Model

MAOS uses a **maintainer-led, open contribution** model. The project is
stewarded by a core maintainer team that makes final decisions on direction,
architecture, and releases. All contributions are welcome through the
standard pull request process.

## Roles

| Role | Responsibility |
|------|---------------|
| **Maintainer** | Merge authority, release management, architecture stewardship, ADR ratification |
| **Reviewer** | Review, story validation, party-mode participation |
| **Contributor** | Pull requests, issue reports, documentation, RFC proposals |

## Decision-Making

| Scope | Mechanism | Record |
|-------|-----------|--------|
| Architecture decisions | ADR process (`docs/adr/`) | Binding ADR document |
| Feature proposals | RFC process (`RFC_TEMPLATE.md`) | RFC document |
| Contentious decisions | Party-mode (multi-agent review panel) | Meeting record in story file |
| Constitutional invariants | Amendment process per ADR-037 | ADR + invariant-lock CI gate |

## Sustainability

The MAOS project intends to establish an Open Collective for transparent
sustainability funding. At launch this is a declared intent accepting $0
expected contributions; fiscal sponsor work is tracked as an initiated item.

## Amendment Process

Changes to this governance document follow the constitutional amendment
process defined in ADR-037. Amendments require maintainer consensus and
are recorded as a new ADR referencing this document.

## Contact

- GitHub: [github.com/lunarpulse/maos](https://github.com/lunarpulse/maos)
- Issues: Use GitHub Issues for bug reports and feature requests
- Security: See `SECURITY.md` for vulnerability reporting
