#!/usr/bin/env bash
# Everything beyond the gate: MSRV, feature powerset, conformance suites,
# examples, workflow lint, coverage. Run inside `devenv shell` (it provides the
# MSRV toolchain and the suite paths). Usage: scripts/test-all.sh [step...]
set -euo pipefail

cd "$(dirname "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")")"

steps=(check actionlint msrv features conformance examples benchmarks coverage)

need() {
  [[ -n ${!1:-} ]] || {
    echo "test-all: $1 is not set; run inside 'devenv shell'" >&2
    exit 1
  }
}

step_check() {
  scripts/check.sh
}

step_actionlint() {
  actionlint
}

step_msrv() {
  need FORGECONF_MSRV_TOOLCHAIN
  PATH="$FORGECONF_MSRV_TOOLCHAIN/bin:$PATH" CARGO_TARGET_DIR=target/msrv \
    cargo test --workspace --all-features --locked
}

step_features() {
  # -D warnings: every feature combination must also build without dead code.
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=target/hack \
    cargo hack test -p forgeconf --feature-powerset --locked
}

step_conformance() {
  need FORGECONF_TOML_TEST_DIR
  need FORGECONF_JSON_TEST_DIR
  need FORGECONF_YAML_TEST_DIR
  cargo test -p forgeconf --all-features --locked \
    --test conformance_toml --test conformance_json --test conformance_yaml -- --ignored
}

# expect <exit code> <stdout+stderr substring> <command...>
expect() {
  local code=$1 want=$2 out status=0
  shift 2
  out=$("$@" 2>&1) || status=$?
  if [[ $status != "$code" || $out != *"$want"* ]]; then
    printf 'test-all: %s\n  exit %s (want %s)\n  output:\n%s\n  want substring: %s\n' \
      "$*" "$status" "$code" "$out" "$want" >&2
    exit 1
  fi
  echo "ok: $*"
}

step_examples() {
  (
    cd forgeconf
    cargo build -q --all-features --examples --locked
    local example
    for example in basic nested validators multiple_formats error_handling enums watch; do
      expect 0 "" cargo run -q --all-features --locked --example "$example"
    done
    local server=(cargo run -q --locked --example server --)
    expect 0 "Usage: server [OPTIONS]" "${server[@]}" --help
    expect 0 "  -p, --port <u16>             Port to listen on [default: 8080] [env: SERVER_PORT]" \
      "${server[@]}" --help
    expect 0 "--log <debug|info|warn>" "${server[@]}" --help
    expect 0 'port: 9000' env SERVER_PORT=9000 "${server[@]}" --tags a --tags b
    expect 0 'host: "0.0.0.0"' "${server[@]}" --tags a --tags b
    expect 0 'log: Warn' "${server[@]}"
    expect 2 "error: port: must be between 1024 and 65535 (cli:--port)" "${server[@]}" --port 80
    expect 2 "error: unknown flag --prot, did you mean --port?" "${server[@]}" --prot 1
  )
}

step_coverage() {
  # Same threshold as the CI coverage job.
  cargo llvm-cov --workspace --all-features --locked --fail-under-lines 80 --summary-only
}

step_benchmarks() {
  # Compiles every comparison bench and runs each once with its correctness
  # checks; no timing. `scripts/bench.sh` takes the measurements.
  cargo bench --manifest-path benchmarks/Cargo.toml --locked -p forgeconf-benchmarks \
    --bench compare -- --test
  FOOTPRINT_QUICK=1 scripts/bench.sh footprint
}

selected=("$@")
((${#selected[@]})) || selected=("${steps[@]}")
for s in "${selected[@]}"; do
  [[ " ${steps[*]} " == *" $s "* ]] || {
    echo "test-all: unknown step '$s' (steps: ${steps[*]})" >&2
    exit 1
  }
done
for s in "${selected[@]}"; do
  echo "== $s"
  "step_$s"
done
echo "test-all: all steps passed"
