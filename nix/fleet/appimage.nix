# fleet: entry point bundled into the fleet AppImage (see FLEET.md, "AppImage").
#
# Nix-built binaries can't load the host's GPU drivers outside NixOS (the driver's own
# dependencies, e.g. libdrm_amdgpu, aren't on Nix's library path), which leaves Zed with
# "Failed to create surface". Pointing the Vulkan loader at Mesa's drivers from the same
# closure, as nixGL does, makes the image independent of the host's driver stack.
{ pkgs, zed-editor }:
pkgs.writeShellApplication {
  name = "zed";
  text = ''
    driver_files=(${pkgs.mesa}/share/vulkan/icd.d/*.json)
    VK_DRIVER_FILES="$(IFS=:; echo "''${driver_files[*]}")"
    # Loaders older than 1.3.234 only read the deprecated name.
    export VK_DRIVER_FILES VK_ICD_FILENAMES="$VK_DRIVER_FILES"
    exec ${zed-editor}/bin/zed "$@"
  '';
}
