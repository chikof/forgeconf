#!/usr/bin/env bash
# Comparison benchmarks (benchmarks/, a separate cargo workspace).
#
#   scripts/bench.sh [runtime|footprint|instructions|charts]... [-- <criterion args>]
#
# runtime    criterion run of benchmarks/runtime (args after `--` go to criterion)
# footprint  clean build time, stripped size and crate count of one minimal
#            binary per library; every binary is run and verified first, so a
#            broken binary never gets a number; written to
#            benchmarks/results/footprint.md and footprint.tsv
# instructions  instructions per iteration of every scenario under callgrind:
#            deterministic, so small changes show without timing noise; written
#            to benchmarks/results/instructions.tsv
# charts     render the SVG charts in benchmarks/images/ from the criterion
#            output and benchmarks/results/*.tsv (run the targets above first)
#
# With no target: runtime, footprint, charts.
#
# FOOTPRINT_RUNS=N   clean builds per library (default 3)
# FOOTPRINT_QUICK=1  no timing: incremental build in a shared target dir and
#                    verification only (CI-style check)
set -euo pipefail

cd "$(dirname "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")")"

MANIFEST=benchmarks/Cargo.toml
FIXTURE=benchmarks/fixtures/app.toml
RESULTS=benchmarks/results/footprint.md
RESULTS_TSV=benchmarks/results/footprint.tsv
RUNS=${FOOTPRINT_RUNS:-3}
QUICK=${FOOTPRINT_QUICK:-0}

LIBS=(forgeconf forgeconf-cli config figment confique serde-toml clap)

die() {
  echo "bench.sh: $*" >&2
  exit 1
}

usage() {
  die "usage: scripts/bench.sh [runtime|footprint|instructions|charts]... [-- <criterion args>]"
}

charts() {
  cargo run -q --release --locked --manifest-path "$MANIFEST" -p forgeconf-charts -- "$PWD/benchmarks"
}

runtime() {
  cargo bench --manifest-path "$MANIFEST" -p forgeconf-benchmarks --bench compare "$@"
}

SCENARIOS=(
  toml-small:forgeconf toml-small:serde toml-large:forgeconf toml-large:serde
  json-small:forgeconf json-small:serde json-large:forgeconf json-large:serde
  yaml-small:forgeconf yaml-small:serde yaml-large:forgeconf yaml-large:serde
  layered:forgeconf layered:confique errors:forgeconf errors:serde cli:forgeconf cli:clap
)

# Instructions one callgrind run of `profile <scenario> <lib> <n>` executes.
callgrind_ir() {
  local out
  out=$(mktemp)
  valgrind --tool=callgrind --callgrind-out-file="$out" \
    benchmarks/target/release/profile "$1" "$2" "$3" >/dev/null 2>&1
  callgrind_annotate "$out" | awk '/PROGRAM TOTALS/ { gsub(",", "", $1); print $1 }'
  rm -f "$out"
}

instructions() {
  local n=${INSTRUCTIONS_ITERS:-200} entry scenario lib base total
  cargo build --release --locked -q -p forgeconf-benchmarks --bin profile --manifest-path "$MANIFEST"
  mkdir -p benchmarks/results
  for entry in "${SCENARIOS[@]}"; do
    scenario=${entry%%:*}
    lib=${entry##*:}
    # Setup (fixture generation, env) is subtracted with a zero-iteration run.
    base=$(callgrind_ir "$scenario" "$lib" 0)
    total=$(callgrind_ir "$scenario" "$lib" "$n")
    printf '%s\t%s\t%d\n' "$scenario" "$lib" $(((total - base) / n))
  done | tee benchmarks/results/instructions.tsv
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

# Median, min and max of the numeric seconds on stdin, space separated.
summarize() {
  sort -n | awk '
    { v[NR] = $1 }
    END {
      m = (NR % 2) ? v[(NR + 1) / 2] : (v[NR / 2] + v[NR / 2 + 1]) / 2
      printf "%.3f %.3f %.3f", m, v[1], v[NR]
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

  local rows="" rows_tsv="" name dir t0 t1 times bin bytes med lo hi kib crates_normal crates_build
  local ctx_rustc ctx_cores ctx_date
  ctx_rustc=$(rustc -V)
  ctx_cores=$(nproc)
  ctx_date=$(date +%F)
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

    read -r med lo hi <<<"$(printf '%s' "$times" | summarize)"
    kib=$(awk -v b="$bytes" 'BEGIN { printf "%.1f", b / 1024 }')
    crates_normal=$(crate_count "$name" normal)
    crates_build=$(crate_count "$name" normal,build)
    rows+="| $name | $(awk -v m="$med" -v a="$lo" -v b="$hi" 'BEGIN { if (a == b) printf "%.1f", m; else printf "%.1f (%.1f-%.1f)", m, a, b }') | $kib | $crates_normal | $crates_build |"$'\n'
    rows_tsv+="$name"$'\t'"$med"$'\t'"$lo"$'\t'"$hi"$'\t'"$kib"$'\t'"$crates_normal"$'\t'"$crates_build"$'\n'
    echo "fp-$name: verified" >&2
  done

  mkdir -p "$(dirname "$RESULTS")"
  {
    echo "Footprint: $ctx_rustc, $ctx_cores cores, $ctx_date, $RUNS clean release builds each"
    echo
    echo "| library | build s median (min-max) | stripped KiB | crates (normal) | crates (normal+build) |"
    echo "|---|---|---|---|---|"
    printf '%s' "$rows"
  } | tee "$RESULTS"
  {
    printf '# rustc=%s\tcores=%s\tdate=%s\truns=%s\tjobs=%s\n' "$ctx_rustc" "$ctx_cores" "$ctx_date" "$RUNS" "${CARGO_BUILD_JOBS:-$ctx_cores}"
    printf 'library\tbuild_s_median\tbuild_s_min\tbuild_s_max\tstripped_kib\tcrates_normal\tcrates_build\n'
    printf '%s' "$rows_tsv"
  } >"$RESULTS_TSV"
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
((${#targets[@]})) || targets=(runtime footprint charts)

for target in "${targets[@]}"; do
  case $target in
    runtime)
      if ((${#extra[@]})); then runtime -- "${extra[@]}"; else runtime; fi
      ;;
    footprint) footprint ;;
    instructions) instructions ;;
    charts) charts ;;
    *) usage ;;
  esac
done
