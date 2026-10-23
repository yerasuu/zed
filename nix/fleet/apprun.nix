# fleet: AppRun for the fleet AppImage (see FLEET.md, "AppImage"). It is nix-appimage's
# userns-chroot AppRun (appruns/userns-chroot/main.c, MIT) changed to keep the host's /nix
# visible instead of replacing it with the bundled store.
{ runCommandCC }:

runCommandCC "AppRun" { } ''
  mkdir $out
  mkdir $out/mountroot
  $CC ${./apprun.c} -o $out/AppRun
''
