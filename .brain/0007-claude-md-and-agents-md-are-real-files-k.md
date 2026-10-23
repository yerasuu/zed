# CLAUDE.md and AGENTS.md are real files (kb), upstream has symlinks

- id: 7
- kind: note
- tags: fleet, upstream-sync
- created: 2026-10-23T16:36:56+02:00

## Description

kb init replaced the upstream symlinks (CLAUDE.md, AGENTS.md -> .rules) with real files that carry kanbrain's section; needed for board management, so they stay.
Effect: merging upstream/main can conflict on both files (FLEET.md says not to edit CLAUDE.md, this is the exception). On conflict keep ours plus any upstream content change, and recheck the kb section is intact.

## Comments

