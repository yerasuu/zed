# Fleet changes stay additive and cheap to merge

- id: 5
- kind: decision
- tags: fleet
- created: 2026-10-23T16:23:02+02:00

## Description

Rules (from FLEET.md): logic in new files or crates; edits to upstream files are 1-5 line hooks marked `// fleet:` (`git grep "// fleet:"` lists them); prefer new fields/variants/registrations over edited lines; never add steps to an upstream sqlez Domain, use a fleet-owned one; do not edit CLAUDE.md, .rules or upstream docs. Merge upstream, never rebase.
Why: the fork tracks zed-industries/zed main and must merge without conflicts.

## Comments

