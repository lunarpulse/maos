# 0.4 Document Conventions

Two conventions govern the document's internal structure. Both are mechanical and cheap to enforce.

**Convention 1 — Single sub-section numbering.** A section containing exactly one numbered subsection numbers it `.1` (not `.2`, `.4`, or any other index). If a future revision introduces additional siblings, the existing `.1` is preserved and new ones extend the sequence (`.2`, `.3`, …). Never use a non-`.1` index for a solitary child — it implies missing siblings the reader will hunt for and not find. **Bold-prose paragraph blocks within the same section are *not* counted as siblings for this rule** — they are informal structure (sticky notes within the chapter, not numbered sections).

**Convention 3 — Promotion rule for bold-prose blocks.** If a bold-prose paragraph block is referenced from outside its parent section (cross-reference, TOC entry, or boundary manifest), it must be promoted to a numbered subsection. Once promoted, sibling-counting under Convention 1 applies normally. The teaching analogy: numbers are addresses, not emphasis. The moment something needs to be *addressable from elsewhere*, it earns a number; if it stays purely structural-within-its-parent-section, it stays bold-prose. Example: §7.2.1.a/b were promoted from bold-prose to real `#####` headings during remediation pass 3 because they became cross-referenced from §3.2.1 and §13.

**Convention 2 — Body↔Appendix dedup signposting.** When a normative specification lives in the body and its derivation, rationale, or worked examples live in an appendix, each side carries one explicit pointer to the other, using fixed prose patterns:

- **Body side (normative home), placed immediately after the table/values:** *"For the derivation of [these values / this specification], see Appendix [X.Y]."*
- **Appendix side (derivation home), as the opening sentence:** *"This appendix derives [the values / the specification] whose normative current-version specification appears in §[N.M] ([Section Title], [Table/Figure ID if applicable]). Reference §[N.M] for the values that govern conformance; this appendix explains how they were chosen [and how to re-derive them when [trigger condition]]."*

The two sentences are reciprocal: each names the other's location, identifies which side is normative, and tells the reader why they would visit the non-current location. Examples in this document: §9.5 ↔ App-F.5; §13.1 ↔ App-D.2.
