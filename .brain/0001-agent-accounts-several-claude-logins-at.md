# Agent accounts: several Claude logins at once

- id: 1
- status: done
- tags: fleet, agent-accounts
- created: 2026-10-23T16:23:02+02:00

## Description

Goal: run Claude Agent under several accounts at once in Zed's agent panel.
Done: `agent: add claude account` action creates `<zed config>/agent_accounts/claude-acp/account-<n>` with symlinks to ~/.claude (settings, CLAUDE.md, skills, commands, agents, projects, history) and a `claude-acp@account-<n>` entry in settings.json. Removal and failed adds delete the account's files. Local projects only (remote_server is stock).
Code: crates/project/src/agent_server_store/agent_accounts.rs, hook in agent_server_store.rs and agent_servers/src/custom.rs. Docs: FLEET.md.

## Comments

## History

- 2026-10-23T16:23:02+02:00 todo -> done (agent)
