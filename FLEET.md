# Fleet branch

`fleet/main` is a long-lived branch that adds Orca-style multi-agent features on top of
upstream Zed. It tracks `zed-industries/zed` `main` and must stay cheap to merge.

## Syncing with upstream

```sh
git remote add upstream https://github.com/zed-industries/zed.git   # once
git config rerere.enabled true                                     # once
git fetch upstream
git switch fleet/main
git merge upstream/main
```

Merge, never rebase: the branch is shared. Sync often; small merges conflict less.
`.github/workflows/fleet_upstream_sync.yml` runs a daily trial merge. GitHub only runs
scheduled workflows from the default branch, so that file must also exist there.

## Building and running

```sh
nix --extra-experimental-features 'nix-command flakes' develop -c cargo build --release -p zed
./target/release/zed
```

Source builds use the `dev` release channel, so the fleet build keeps its own database and
never shares one with an installed stable Zed. `settings.json` is shared between channels.

## Rules for fleet changes

- Put logic in new files or new crates. New files never conflict.
- Edits to upstream files are small hooks (1-5 lines) that call into fleet code. Mark each
  one with a `// fleet:` comment; `git grep "// fleet:"` lists every hook to recheck.
- Prefer additive changes (new field, new variant, new registration) over editing lines.
- Never add migration steps to an upstream `sqlez` `Domain`: steps are matched by index,
  so an upstream step at the same index breaks existing databases. Use a fleet-owned
  `Domain` instead.
- Do not edit `CLAUDE.md`, `.rules` or upstream docs; document fleet behavior here.

## Features

### Agent accounts

Run the same registry agent under several accounts at once. An `agent_servers` entry keyed
`<agent>@<account>` runs registry agent `<agent>` as its own external agent: its own
process, its own entry in the new-thread menu (shown as "Claude Agent (work)") and its own
threads. No settings schema change is involved, so stock Zed simply ignores these entries.

```json
"agent_servers": {
  "claude-acp": { "type": "registry" },
  "claude-acp@work": {
    "type": "registry",
    "env": { "CLAUDE_CONFIG_DIR": "/home/me/.claude-work" }
  },
  "claude-acp@personal": {
    "type": "registry",
    "env": { "CLAUDE_CONFIG_DIR": "/home/me/.claude-personal" }
  }
}
```

Point each account at its own config dir via the agent's variable (`CLAUDE_CONFIG_DIR` for
Claude Code, `CODEX_HOME` for Codex) so each keeps its own login; log in once per account.
Account entries get the same agent-specific env as the base agent (for example Claude's
empty `ANTHROPIC_API_KEY`, which keeps subscription login in use). Each account installs
the agent into its own directory.

`script/fleet-add-account <name>` creates a Claude account: it makes `~/.claude-<name>`,
symlinks `settings.json`, `keybindings.json`, `CLAUDE.md`, `skills`, `commands`, `agents`,
`projects` and `history.jsonl` from `~/.claude` (the approach `claude-swap` uses), and adds
`claude-acp@<name>` to Zed's `settings.json`. Accounts therefore share settings and
conversation history; only the login differs. `.credentials.json` and `.claude.json` are
never linked because they hold the account's login.

Limitations: accounts only work in local projects, because remote projects run the stock
`remote_server`. The code lives in `crates/project/src/agent_server_store/agent_accounts.rs`.
