# Agent Accounts page with login, plan and usage

- id: 2
- status: done
- tags: fleet, agent-accounts, ui
- created: 2026-10-23T16:23:02+02:00

## Description

Goal: see every account's state in one page.
Done: page lists running state, login, email, org, role, plan and 5h/7d usage with reset times (read from Claude Code's .claude.json cache, refreshed every 30 s), with add/start-thread/remove buttons and cleanup of orphan directories. Reachable from `agent: open accounts` and the `+` menu of the agent panel.
Code: crates/agent_ui/src/fleet_accounts.rs and fleet_accounts/accounts_page.rs.

## Comments

## History

- 2026-10-23T16:23:02+02:00 todo -> done (agent)
