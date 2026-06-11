#!/usr/bin/env bash
# ori-derive: resolve upstream sections and compute hashes.
#
# Usage: resolve-upstream.sh <slice-id> [--manifest <path>] [--project-root <dir>]
#
# Reads `.ori/slices/<slice-id>/manifest.yaml` (or --manifest path) and emits
# one line per derives_from entry:
#
#   <ref> <sha-prefix>          (file resolved, hash computed)
#   <ref> NOT_FOUND  (to stderr) (file missing)
#
# where <ref> is either "<file>#<section>" (structured form or `#`-delimited
# string form) or "<file>" (no section). Both manifest forms below are
# supported (ori-szx):
#
#   # Legacy string form — one line per entry:
#   derives_from:
#     - domain/aggregates.md#note-aggregate
#     - domain/workflows/capture-auto-save.md
#
#   # Structured form — two-line entry with path: + optional section:
#   derives_from:
#     - path: .ori/domain/bounded-contexts.md
#       section: task-management
#     - path: .ori/domain/aggregates.md
#
# Path resolution: if <file> starts with `.ori/` it is treated as
# project-root-relative; otherwise the script prepends `.ori/`. This lets
# both styles co-exist while we settle on one in the skill SSoT.
#
# Exit codes:
#   0  success (some entries may be NOT_FOUND — they print to stderr only)
#   1  manifest missing / invalid args / project root not found
#   2  usage error (unknown flag)
set -euo pipefail

MANIFEST_ARG=""
PROJECT_ROOT_ARG=""
ID=""

usage() {
  cat >&2 <<'EOF'
Usage: resolve-upstream.sh <slice-id> [options]

Options:
  --manifest <path>       Override manifest.yaml location (skip slice-id lookup)
  --project-root <dir>    Override project root (default: git rev-parse / .ori/ ancestor)
  -h, --help              Show this help and exit
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)      MANIFEST_ARG="${2:-}";      shift 2 ;;
    --project-root)  PROJECT_ROOT_ARG="${2:-}";  shift 2 ;;
    -h|--help)       usage; exit 0 ;;
    --)              shift; break ;;
    -*) echo "ERROR: unknown flag: $1" >&2; usage; exit 2 ;;
    *)
      if [[ -z "$ID" ]]; then
        ID="$1"; shift
      else
        echo "ERROR: unexpected positional arg: $1" >&2; usage; exit 2
      fi
      ;;
  esac
done

# Resolve PROJECT_ROOT
if [[ -n "$PROJECT_ROOT_ARG" ]]; then
  PROJECT_ROOT="$PROJECT_ROOT_ARG"
else
  SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
  PROJECT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -z "$PROJECT_ROOT" ]]; then
    d="$SCRIPT_DIR"
    while [[ "$d" != "/" ]]; do
      if [[ -d "$d/.ori" ]]; then PROJECT_ROOT="$d"; break; fi
      d="$(dirname "$d")"
    done
  fi
fi

if [[ -z "$PROJECT_ROOT" || ! -d "$PROJECT_ROOT" ]]; then
  echo "ERROR: cannot find project root (.ori/ not found)" >&2
  exit 1
fi
PROJECT_ROOT="$(cd "$PROJECT_ROOT" && pwd)"

# Resolve MANIFEST
if [[ -n "$MANIFEST_ARG" ]]; then
  MANIFEST="$MANIFEST_ARG"
else
  if [[ -z "$ID" ]]; then
    echo "ERROR: slice-id required (or pass --manifest)" >&2
    usage
    exit 1
  fi
  MANIFEST="$PROJECT_ROOT/.ori/slices/$ID/manifest.yaml"
fi

if [[ ! -f "$MANIFEST" ]]; then
  echo "ERROR: $MANIFEST not found" >&2
  exit 1
fi

# Parse derives_from. awk emits one "ref" per line; the loop below resolves
# the file and computes its sha256 prefix.
parse_refs() {
  awk '
    BEGIN { in_derives = 0; current_path = ""; current_section = "" }

    # Flush the buffered structured entry, if any.
    function flush() {
      if (current_path == "") return
      if (current_section != "")
        print current_path "#" current_section
      else
        print current_path
      current_path = ""
      current_section = ""
    }

    # End of derives_from when a top-level key starts (no leading space).
    in_derives && /^[A-Za-z_]/ {
      flush()
      in_derives = 0
    }

    /^derives_from:[[:space:]]*$/ { in_derives = 1; next }

    # Structured form: "  - path: <p>" (must be checked before legacy form).
    in_derives && /^[[:space:]]*-[[:space:]]+path:[[:space:]]*/ {
      flush()
      sub(/^[[:space:]]*-[[:space:]]+path:[[:space:]]*/, "")
      gsub(/^"|^'\''|"$|'\''$/, "")
      current_path = $0
      next
    }

    # Continuation line: "    section: <s>" — attaches to the buffered entry.
    in_derives && /^[[:space:]]+section:[[:space:]]*/ && current_path != "" {
      sub(/^[[:space:]]+section:[[:space:]]*/, "")
      gsub(/^"|^'\''|"$|'\''$/, "")
      current_section = $0
      next
    }

    # Legacy string form: "  - <ref>" (one line, may contain "#section").
    in_derives && /^[[:space:]]*-[[:space:]]+[^[:space:]]/ {
      flush()
      sub(/^[[:space:]]*-[[:space:]]+/, "")
      gsub(/^"|^'\''|"$|'\''$/, "")
      print
      next
    }

    END { flush() }
  ' "$1"
}

resolve_path() {
  # Translate a ref (file or file#section) into an on-disk path.
  local ref="$1"
  local file="${ref%%#*}"
  if [[ "$file" = .ori/* || "$file" = /* ]]; then
    printf '%s\n' "$file"
  else
    printf '%s\n' ".ori/$file"
  fi
}

# Emit results (one ref per line: "<ref> <sha-prefix>" on stdout, NOT_FOUND
# entries on stderr). Iterate via a file descriptor so the loop runs in the
# parent shell — no subshell — making future state additions (e.g. error
# counters) work as expected.
while IFS= read -r ref; do
  [[ -z "$ref" ]] && continue
  rel="$(resolve_path "$ref")"
  abs="$PROJECT_ROOT/$rel"
  if [[ -f "$abs" ]]; then
    hash="$(sha256sum "$abs" | cut -c1-12)"
    echo "$ref $hash"
  else
    echo "$ref NOT_FOUND" >&2
  fi
done < <(parse_refs "$MANIFEST")
