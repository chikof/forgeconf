#!/usr/bin/env bash
# Print the memory layout of this workspace's own types, largest first, with field and variant lines.
#
# Usage: scripts/type-sizes.sh [TYPE_FILTER]
#   TYPE_FILTER  substring; keeps only types whose name contains it. Without a filter only types
#                defined in this workspace are listed. With one, wrappers of them match too, so
#                `scripts/type-sizes.sh 'Option<'` shows every Option around a workspace type.
#
# RUSTC_BOOTSTRAP=1 makes the stable rustc accept the unstable -Zprint-type-sizes flag. This is for
# local measurement only: the output format is unstable, so this is NOT part of `check` or CI.
# It builds into target/type-sizes so the normal build cache stays valid. Workspace crates are
# cleaned first, because a cached build prints nothing.
set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null || dirname "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")")"

filter="${1:-}"
target_dir=target/type-sizes

mapfile -t crates < <(cargo metadata --no-deps --format-version 1 |
  grep -o '"name":"[^"]*","version"' | sed 's/^"name":"//; s/","version"$//')
clean_args=()
for crate in "${crates[@]}"; do clean_args+=(-p "$crate"); done
cargo clean -q --target-dir "$target_dir" "${clean_args[@]}"

# Crate names as rustc prints them in type paths: `-` becomes `_`.
prefixes=$(printf '%s\n' "${crates[@]//-/_}" | paste -sd'|')

RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Zprint-type-sizes \
  cargo build --workspace --lib --bins --target-dir "$target_dir" 2>&1 |
  awk -v prefixes="$prefixes" -v filter="$filter" '
    function flush() {
      if (name != "" && !(block in seen)) {
        seen[block] = 1
        gsub(/\n/, "\001", block)
        printf "%d\t%s\t%s\n", size, name, block
      }
      name = ""
    }
    /^print-type-size type: / {
      flush()
      line = $0
      sub(/^print-type-size type: `/, "", line)
      name = line; sub(/`: .*/, "", name)
      size = line; sub(/^.*`: /, "", size); sub(/ bytes.*/, "", size)
      own = (name ~ ("^(" prefixes ")::"))
      wrapped = (name ~ ("(^|[^A-Za-z0-9_:])(" prefixes ")::"))
      keep = (filter == "") ? own : (wrapped && index(name, filter) > 0)
      if (!keep) { name = "" ; next }
      block = $0
      next
    }
    name != "" && /^print-type-size / { block = block "\n" $0; next }
    { flush() }
    END { flush() }
  ' |
  sort -t$'\t' -s -k1,1nr -k2,2 |
  cut -f3 | tr '\001' '\n' | sed 's/^print-type-size //'
