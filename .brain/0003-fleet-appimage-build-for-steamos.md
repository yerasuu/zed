# Fleet AppImage build for SteamOS

- id: 3
- status: done
- tags: fleet, build
- created: 2026-10-23T16:23:02+02:00

## Description

Goal: ship the fork as one AppImage that works without nixGL or host GPU drivers.
Done: script/fleet-appimage builds with the repo flake (nix/build.nix) and nix-appimage; nix/fleet/appimage.nix bundles Mesa Vulkan drivers, skips host implicit layers, adds desktop entry and icon for AppManager. Build inputs are rooted at target/fleet-build-cache; per-build store paths are deleted after the copy. Uses the nightly channel (own database). Tested on SteamOS with AMD GPU.

## Comments

## History

- 2026-10-23T16:23:02+02:00 todo -> done (agent)
