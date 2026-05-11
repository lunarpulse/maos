# Appendix C — What this document deliberately is NOT

To save reviewer time later:

- **Not a UI spec.** TUI / editor / mobile shells are application-level work. The kernel's notification primitives are documented; the visual designs are not.
- **Not a benchmark plan.** Performance targets are defined where they are load-bearing; broader benchmarking is out of scope until v0.5 ships and there are real numbers to optimize against.
- **Not a marketing document.** No tagline. No one-pager. Those are downstream.
- **Not a project plan.** §13's roadmap is sequence + validation milestones, not Gantt-able tasks.
- **Not a security audit.** §8 is a threat model and mitigation summary. A real audit happens before v1.0 ships (external pen-test with zero P0/P1 findings open).
- **Not the final answer on names.** "MAOS", "Spirit", "Loom-lite", "Host", "Posture" — all are working names. Renaming is cheap before v0.1.
