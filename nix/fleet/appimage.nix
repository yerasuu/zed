# fleet: package bundled into the fleet AppImage (see FLEET.md, "AppImage").
{ pkgs, zed-editor }:
let
  # Nix-built binaries can't load the host's GPU drivers outside NixOS (the driver's own
  # dependencies, e.g. libdrm_amdgpu, aren't on Nix's library path), which leaves Zed with
  # "Failed to create surface". Pointing the Vulkan loader at Mesa's drivers from the same
  # closure, as nixGL does, makes the image independent of the host's driver stack.
  launcher = pkgs.writeShellApplication {
    name = "zed";
    text = ''
      driver_files=(${pkgs.mesa}/share/vulkan/icd.d/*.json)
      VK_DRIVER_FILES="$(IFS=:; echo "''${driver_files[*]}")"
      # Loaders older than 1.3.234 only read the deprecated name.
      export VK_DRIVER_FILES VK_ICD_FILENAMES="$VK_DRIVER_FILES"
      # Host implicit layers (Mesa device select, overlays) fail to load for the same reason
      # as host drivers, so skip them instead of logging an error on every start.
      export VK_LOADER_LAYERS_DISABLE='~implicit~'
      exec ${zed-editor}/bin/zed "$@"
    '';
  };
in
# AppImage managers such as AppManager integrate an image from the `.desktop` file and icon at
# its root. nix-appimage copies them there from this package, matching the `.desktop` file by
# its `Exec` and taking `.DirIcon` only from icons of 256x256 or smaller.
pkgs.runCommand "zed-fleet-${zed-editor.version}"
  {
    nativeBuildInputs = [ pkgs.imagemagick ];
    meta.mainProgram = "zed";
  }
  ''
    mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor/256x256/apps
    ln -s ${launcher}/bin/zed $out/bin/zed
    magick ${zed-editor}/share/icons/hicolor/512x512/apps/zed.png -resize 256x256 \
      $out/share/icons/hicolor/256x256/apps/zed-fleet.png
    # A distinct name and icon keep it apart from an installed Zed. Without TryExec the entry
    # stays visible even though `zed` isn't on PATH (the manager rewrites Exec to the image).
    # Dropping the second main category keeps it from being listed twice in app menus.
    sed \
      -e '0,/^Name=.*/s//Name=Zed Fleet/' \
      -e 's/^Icon=.*/Icon=zed-fleet/' \
      -e '/^TryExec=/d' \
      -e 's/^Categories=Utility;/Categories=/' \
      -e '/^\[Desktop Entry\]/a X-AppImage-Version=${zed-editor.version}\nStartupWMClass=dev.zed.Zed-Nightly' \
      ${zed-editor}/share/applications/dev.zed.Zed-Nightly.desktop \
      > $out/share/applications/zed-fleet.desktop
  ''
