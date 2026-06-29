---
name: skill-creator
description: "Create new skills, modify and improve existing skills, and measure skill performance. Use when users want to create a skill from scratch, edit, or optimize an existing skill, run evals to test a skill, benchmark skill performance with variance analysis, or optimize a skill's description for better triggering accuracy."
source_url: "https://skills.pub/en/skills/anthropics-skills::example-skills::skill-creator::"
source_observed: 2026-06-25
---

# Skill Creator

A skill for creating new skills and iteratively improving them.

At a high level, the process of creating a skill goes like this:

- Decide what you want the skill to do and roughly how it should do it
- Write a draft of the skill
- Create a few test prompts and run claude-with-access-to-the-skill on them
- Help the user evaluate the results both qualitatively and quantitatively
- Rewrite the skill based on feedback from the user's evaluation of the results
- Repeat until you're satisfied
- Expand the test set and try again at larger scale

## Creating a skill

Start by understanding the user's intent. The current conversation might already
contain a workflow the user wants to capture. If so, extract answers from the
conversation history first — the tools used, the sequence of steps, corrections
the user made, input/output formats observed. The user may need to fill gaps.

Write `SKILL.md` with YAML frontmatter containing a concise `name` and a precise
`description` that explains what the skill does and when it should be used. Keep
the body focused on workflow, edge cases, examples, and references.
