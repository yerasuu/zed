# AppImage: access host system (not fully isolated)

- id: 6
- status: done
- tags: fleet, build
- created: 2026-10-23T16:23:43+02:00

## Description

Goal: the fleet AppImage reaches host resources it needs (PATH tools, git, ssh agent, fonts, certs, shells, xdg portals, home dotfiles) instead of running sealed.
Done when: terminal/git/LSPs/agents inside Zed see host tools and config; GPU still works; verified on SteamOS.

## Comments

### 2026-10-23T16:26:57+02:00 (agent)

Probed with a bash AppImage: host / (usr, run, home, fonts, git) was already visible; host /nix was hidden (no ~/.nix-profile tools, no nix). Added nix/fleet/apprun.{c,nix}: bind host /nix, overlay bundled store over /nix/store (falls back to upstream behaviour). Probe image verified: host profile tools and nix run inside. script/fleet-appimage now builds with it. Remaining limit: userns, so sudo/setuid and extra groups don't work. Full Zed image not built yet (15-20 min).

### 2026-10-23T16:59:10+02:00 (agent)

Full image built (731 MB, target/Zed-fleet-x86_64.AppImage); --version and --help run fine with the new AppRun. Left for the user: open the window and check GPU, fonts, and that the terminal sees ~/.nix-profile tools.

### 2026-10-23T17:01:00+02:00 (agent)

Verified by the user: the image works.

## Activity

- 2026-10-23T16:59:10+02:00 tags (agent): +agent-user
- 2026-10-23T17:01:00+02:00 tags (agent): -agent-user

## History

- 2026-10-23T16:23:43+02:00 todo -> doing (agent)
- 2026-10-23T17:01:00+02:00 doing -> done (agent)

## Sessions

- 2026-10-23T16:23:43+02:00 -> 2026-10-23T17:01:00+02:00 (37m)
