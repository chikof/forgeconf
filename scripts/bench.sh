#!/usr/bin/env bash
# Comparison benchmarks (benchmarks/, a separate cargo workspace).
#
#   scripts/bench.sh [runtime|footprint]... [-- <criterion args>]
#
# runtime    criterion run of benchmarks/runtime (args after `--` go to criterion)
# footprint  clean build time, stripped size and crate count of one minimal
#            binary per library; every binary is run and verified first, so a
#            broken binary never gets a number
#
# FOOTPRINT_RUNS=N   clean builds per library (default 3)
# FOOTPRINT_QUICK=1  no timing: incremental build in a shared target dir and
#                    verification only (CI-style check)
set -euo pipefail

cd "$(dirname "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")")"

MANIFEST=benchmarks/Cargo.toml
FIXTURE=benchmarks/fixtures/app.toml
RESULTS=benchmarks/results/footprint.md
RUNS=${FOOTPRINT_RUNS:-3}
QUICK=${FOOTPRINT_QUICK:-0}

LIBS=(forgeconf forgeconf-cli config figment confique serde-toml clap)

die() {
  echo "bench.sh: $*" >&2
  exit 1
}

usage() {
  die "usage: scripts/bench.sh [runtime|footprint]... [-- <criterion args>]"
}

runtime() {
  cargo bench --manifest-path "$MANIFEST" -p forgeconf-benchmarks --bench compare "$@"
}

# Prints the binary's stdout for the scenario its library is verified with.
run_fp() {
  local bin=$1 name=$2
  case $name in
    forgeconf | confique)
      env APP_SERVER_PORT=9090 APP_DATABASE_POOL_SIZE=32 "$bin" "$FIXTURE"
      ;;
    config | figment)
      env APP_SERVER__PORT=9090 APP_DATABASE__POOL_SIZE=32 "$bin" "$FIXTURE"
      ;;
    serde-toml)
      "$bin" "$FIXTURE"
      ;;
    forgeconf-cli | clap)
      "$bin" --host 10.0.0.1 --port 9090 --debug --features a --features b
      ;;
  esac
}

expected_fp() {
  case $1 in
    serde-toml) echo "port=8080 pool=16 limits=3" ;;
    forgeconf-cli | clap) echo "host=10.0.0.1 port=9090 debug=true features=2" ;;
    *) echo "port=9090 pool=32 limits=3" ;;
  esac
}

verify_fp() {
  local name=$1 bin=$2 got want
  want=$(expected_fp "$name")
  got=$(run_fp "$bin" "$name") || die "fp-$name exited non-zero"
  [[ $got == "$want" ]] || die "fp-$name printed '$got', expected '$want'"
}

build_fp() {
  local name=$1 dir=$2
  cargo build --release --locked -q -p "fp-$name" --manifest-path "$MANIFEST" --target-dir "$dir"
}

now() {
  date +%s.%N
}

# Median and min-max of the sorted-numerically seconds on stdin.
summarize() {
  sort -n | awk '
    { v[NR] = $1 }
    END {
      m = (NR % 2) ? v[(NR + 1) / 2] : (v[NR / 2] + v[NR / 2 + 1]) / 2
      if (NR == 1) printf "%.1f", m
      else printf "%.1f (%.1f-%.1f)", m, v[1], v[NR]
    }'
}

crate_count() {
  cargo tree --manifest-path "$MANIFEST" -p "fp-$1" -e "$2" --prefix none |
    sed 's/ (.*//' | sort -u | wc -l
}

footprint_quick() {
  local dir=benchmarks/target/footprint-check name
  for name in "${LIBS[@]}"; do
    build_fp "$name" "$dir"
    verify_fp "$name" "$dir/release/fp-$name"
    echo "fp-$name: ok"
  done
}

footprint() {
  if [[ $QUICK == 1 ]]; then
    footprint_quick
    return
  fi
  [[ $RUNS =~ ^[1-9][0-9]*$ ]] || die "FOOTPRINT_RUNS must be a positive integer"

  local rows="" name dir t0 t1 times bin bytes
  # Global: the EXIT trap runs after this function's locals are gone.
  footprint_tmp=$(mktemp -d)
  trap 'rm -rf "$footprint_tmp"' EXIT
  local tmp=$footprint_tmp

  for name in "${LIBS[@]}"; do
    dir=benchmarks/target/footprint/$name
    times=""
    for ((i = 1; i <= RUNS; i++)); do
      rm -rf "$dir"
      t0=$(now)
      build_fp "$name" "$dir"
      t1=$(now)
      times+=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f\n", b - a }')$'\n'
    done

    bin=$dir/release/fp-$name
    verify_fp "$name" "$bin"

    cp "$bin" "$tmp/fp-$name"
    strip "$tmp/fp-$name"
    bytes=$(stat -c %s "$tmp/fp-$name")

    rows+="| $name | $(printf '%s' "$times" | summarize) | $(awk -v b="$bytes" 'BEGIN { printf "%.1f", b / 1024 }') | $(crate_count "$name" normal) | $(crate_count "$name" normal,build) |"$'\n'
    echo "fp-$name: verified" >&2
  done

  mkdir -p "$(dirname "$RESULTS")"
  {
    echo "Footprint: $(rustc -V), $(nproc) cores, $(date +%F), $RUNS clean release builds each"
    echo
    echo "| library | build s median (min-max) | stripped KiB | crates (normal) | crates (normal+build) |"
    echo "|---|---|---|---|---|"
    printf '%s' "$rows"
  } | tee "$RESULTS"
}

targets=()
extra=()
while (($#)); do
  if [[ $1 == -- ]]; then
    shift
    extra=("$@")
    break
  fi
  targets+=("$1")
  shift
done
((${#targets[@]})) || targets=(runtime footprint)

for target in "${targets[@]}"; do
  case $target in
    runtime)
      if ((${#extra[@]})); then runtime -- "${extra[@]}"; else runtime; fi
      ;;
    footprint) footprint ;;
    *) usage ;;
  esac
done
