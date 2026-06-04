{pkgs, ...}: {
  languages.rust = {
    enable = true;
    # rustfmt.toml uses nightly-only options and CI formats on nightly.
    channel = "nightly";
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-src" "rust-analyzer" "llvm-tools-preview"];
  };

  packages = [
    pkgs.git
    pkgs.shellcheck
    pkgs.typos
    pkgs.cargo-deny
    pkgs.cargo-nextest
    pkgs.bacon
    pkgs.git-cliff
    pkgs.cargo-edit
    # its own test suite fails in the nix sandbox
    (pkgs.cargo-llvm-cov.overrideAttrs {doCheck = false;})
    pkgs.cargo-expand
    pkgs.grcov
    pkgs.llvmPackages_19.libllvm
  ];

  env.LAZYVIM_RUST_DIAGNOSTICS = "bacon-ls";

  dotenv.enable = true;

  # scripts/check.sh is the single gate.
  scripts.check.exec = ''
    exec "$DEVENV_ROOT/scripts/check.sh"
  '';

  enterShell = ''
    echo "forgeconf: run 'check'"
  '';
}
