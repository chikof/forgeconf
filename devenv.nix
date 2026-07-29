{
  pkgs,
  inputs,
  ...
}: let
  # Kept off PATH so it never shadows the nightly toolchain; `msrv` uses it.
  msrvToolchain = (inputs.rust-overlay.lib.mkRustBin {} pkgs).stable."1.85.0".minimal;

  # Conformance suites, read by forgeconf/tests/conformance_*.rs (`conformance`).
  tomlTest = pkgs.runCommand "toml-test-1.0-files" {} ''
    ${pkgs.toml-test}/bin/toml-test copy -toml 1.0 $out
  '';
  jsonTestSuite = pkgs.fetchFromGitHub {
    owner = "nst";
    repo = "JSONTestSuite";
    rev = "1ef36fa01286573e846ac449e8683f8833c5b26a";
    hash = "sha256-s2yMgVWq2DwibAjOvLKhGDbEwXm4yke/g4mp7u565H4=";
  };
  yamlTestSuite = pkgs.fetchFromGitHub {
    owner = "yaml";
    repo = "yaml-test-suite";
    rev = "6e6c296ae9c9d2d5c4134b4b64d01b29ac19ff6f"; # tag data-2022-01-17
    hash = "sha256-zXJy/dvbxkR47gWcfD4iN8mdr2gGR/L37KbcdYy3rAM=";
  };
in {
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
    pkgs.cargo-hack
    pkgs.actionlint
    pkgs.bacon
    pkgs.git-cliff
    pkgs.cargo-edit
    # its own test suite fails in the nix sandbox
    (pkgs.cargo-llvm-cov.overrideAttrs {doCheck = false;})
    pkgs.cargo-expand
    pkgs.grcov
    pkgs.llvmPackages_19.libllvm
  ];

  env = {
    LAZYVIM_RUST_DIAGNOSTICS = "bacon-ls";
    FORGECONF_MSRV_TOOLCHAIN = "${msrvToolchain}";
    FORGECONF_TOML_TEST_DIR = "${tomlTest}";
    FORGECONF_JSON_TEST_DIR = "${jsonTestSuite}/test_parsing";
    FORGECONF_YAML_TEST_DIR = "${yamlTestSuite}";
  };

  dotenv.enable = true;

  # scripts/check.sh is the single gate; scripts/test-all.sh runs everything;
  # scripts/bench.sh measures forgeconf against other config crates.
  scripts.check.exec = ''
    exec "$DEVENV_ROOT/scripts/check.sh"
  '';
  scripts.test-all.exec = ''
    exec "$DEVENV_ROOT/scripts/test-all.sh" "$@"
  '';
  scripts.bench.exec = ''
    exec "$DEVENV_ROOT/scripts/bench.sh" "$@"
  '';

  enterShell = ''
    echo "forgeconf: run 'check' (gate), 'test-all' (everything) or 'bench'"
  '';
}
