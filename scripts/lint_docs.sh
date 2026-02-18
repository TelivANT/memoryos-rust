#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DOCS_DIR="$ROOT_DIR/docs"

fail() {
  echo "[lint-docs] ERROR: $1" >&2
  exit 1
}

# 1) Ban deprecated header token outside conflict-resolution doc.
if rg -n "X-Status" "$DOCS_DIR" -g '*.md' | grep -v "spec_conflict_resolution.md" >/tmp/lint_docs_xstatus.txt; then
  cat /tmp/lint_docs_xstatus.txt >&2
  fail "deprecated header 'X-Status' found outside spec_conflict_resolution.md"
fi

# 2) Detect duplicate numbered level-2 headings like '## 6.5'.
while IFS= read -r file; do
  dups=$(awk '
    /^## [0-9]+\.[0-9]+/ {
      key=$2;
      count[key]++;
    }
    END {
      for (k in count) if (count[k] > 1) print k;
    }
  ' "$file")
  if [[ -n "$dups" ]]; then
    echo "[lint-docs] Duplicate numbered sections in $file:" >&2
    echo "$dups" >&2
    fail "duplicate section numbering detected"
  fi
done < <(find "$DOCS_DIR" -type f -name '*.md' | sort)

# 3) Ensure ID glossary is present in api standard.
for term in request_id trace_id event_id task_id; do
  if ! rg -q "$term" "$DOCS_DIR/specs/api_standard.md"; then
    fail "missing glossary term '$term' in specs/api_standard.md"
  fi
done

# 4) Basic secret pattern scan in docs.
if rg -n "cpx_[A-Za-z0-9]+|sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}" "$DOCS_DIR" -g '*.md' >/tmp/lint_docs_secrets.txt; then
  cat /tmp/lint_docs_secrets.txt >&2
  fail "potential live secret pattern found in docs"
fi

echo "[lint-docs] OK"
