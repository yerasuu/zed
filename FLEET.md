# Fleet branch

The `main` branch of this fork adds Orca-style multi-agent features on top of upstream Zed.
It tracks `zed-industries/zed` `main` and must stay cheap to merge.

## Syncing with upstream

```sh
git remote add upstream https://github.com/zed-industries/zed.git   # once
git config rerere.enabled true                                     # once
git fetch upstream
git switch main
git merge upstream/main
```

GitHub's "Sync fork" button does the same merge. Merge, never rebase: the branch is shared.
Sync often; small merges conflict less. `.github/workflows/fleet_upstream_sync.yml` runs a
daily trial merge and reports conflicts or breakage without pushing anything.

## Building and running

```sh
nix --extra-experimental-features 'nix-command flakes' develop -c cargo build --release -p zed
./target/release/zed
```

Outside NixOS the Nix-built binary can't use the system GPU driver (its Vulkan loader fails
with `libdrm_amdgpu.so.1: cannot open shared object file`, and Zed reports "Failed to create
surface"). Run it through nixGL, which supplies Nix's Mesa drivers:

```sh
nix --extra-experimental-features 'nix-command flakes' run --impure \
  github:nix-community/nixGL#nixVulkanIntel -- ./target/release/zed <folder>
```

`nixVulkanIntel` ships all of Mesa's Vulkan drivers, AMD's included.

Source builds use the `dev` release channel, so the fleet build keeps its own database and
never shares one with an installed stable Zed. `settings.json` is shared between channels.

### AppImage

```sh
script/fleet-appimage    # writes target/Zed-fleet-x86_64.AppImage
```

The script builds Zed with the repo's flake (`nix/build.nix`) and bundles it with
[nix-appimage](https://github.com/ralismark/nix-appimage). `nix/fleet/appimage.nix` wraps it
with Mesa's Vulkan drivers from the same closure, so the image doesn't need nixGL or the
host's GPU drivers (tested on SteamOS with an AMD GPU). Only committed changes are included, and every
commit rebuilds Zed from scratch (about 15-20 minutes) because the commit SHA is part of the
package. The image is about 850 MB, mostly Mesa and LLVM.

The image meets what AppImage managers such as
[AppManager](https://github.com/kem-a/AppManager) need to add it to the app menu: a SquashFS
payload with `AppRun`, `zed-fleet.desktop` ("Zed Fleet", with `X-AppImage-Version` and
`StartupWMClass=dev.zed.Zed-Nightly`) and a 256x256 PNG `.DirIcon` at its root. Open the
image with AppManager to install it. It has no update information, so AppManager won't
update it; install a newly built image over it instead. The flake builds the `nightly`
channel, so the AppImage has its own database, separate from the `dev` source build.

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

Run Claude Agent under several accounts at once. Run `agent: add claude account` from the
command palette: Zed creates `<zed config dir>/agent_accounts/claude-acp/account-<n>`
(`~/.config/zed/...` on Linux), saves a `claude-acp@account-<n>` entry
in `settings.json` and opens a thread with it, where Claude's login prompt appears. After
logging in once, the account stays in the new-thread menu as "Claude Agent (account-<n>)".

Each account directory links `settings.json`, `keybindings.json`, `CLAUDE.md`, `skills`,
`commands`, `agents`, `projects` and `history.jsonl` from `~/.claude` (the approach
`claude-swap` uses), so accounts share settings and conversation history and only the login
differs. `.credentials.json` and `.claude.json` are never linked because they hold the login.

How it works: an `agent_servers` entry keyed `<agent>@<account>` runs registry agent
`<agent>` as its own external agent, with its own process and threads. No settings schema
change is involved, so stock Zed ignores these entries. Account entries get the base agent's
agent-specific env (for example Claude's empty `ANTHROPIC_API_KEY`, which keeps subscription
login in use). The same mechanism works for other registry agents by hand, e.g.
`"codex-acp@work": { "type": "registry", "env": { "CODEX_HOME": "..." } }`.

`agent: open accounts` (or "Agent Accounts" at the bottom of the agent panel's `+` menu)
opens the Agent Accounts page, which lists every Claude account with
whether its agent is running, whether it is logged in, its email, organization, role and
plan, and its 5-hour and 7-day usage with reset times. Usage comes from the cache Claude Code
keeps in `.claude.json` (no extra API calls); the page rereads it every 30 seconds. Only
non-secret fields of `.credentials.json` are parsed. The page also has buttons to add an
account and to start a thread with a given account.

An account's files exist only while the account does. Removing an account from the page
(trash button, confirmed) deletes its settings entry first and its directory second; the
shared items are symlinks, so only the links go and `~/.claude` is untouched. A failed add
deletes the directory it created. Directories under `agent_accounts/claude-acp/` with no
settings entry (for example after deleting an entry from `settings.json` by hand) are listed
as "Files without an account" with a Delete button. They are not deleted automatically,
because a momentarily invalid `settings.json` would otherwise wipe every account's login.
Only direct children of `agent_accounts/claude-acp/` are ever deleted.

Limitations: accounts only work in local projects, because remote projects run the stock
`remote_server`. Code: `crates/project/src/agent_server_store/agent_accounts.rs` and
`crates/agent_ui/src/fleet_accounts.rs` (with `fleet_accounts/accounts_page.rs`).
