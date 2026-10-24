#!/bin/sh
# Fails if any Rust source file is longer than the limit (tests included).
# Small files that each do one thing are a project rule; split, don't raise
# the limit.
#
#   scripts/check-file-sizes.sh [LIMIT]

set -eu

limit="${1:-250}"
cd "$(dirname "$0")/.."

too_long=$(
    git ls-files --cached --others --exclude-standard -- '*.rs' |
        while IFS= read -r file; do
            [ -f "$file" ] || continue
            lines=$(wc -l <"$file")
            if [ "$lines" -gt "$limit" ]; then
                printf '%6d  %s\n' "$lines" "$file"
            fi
        done | sort -rn
)

if [ -n "$too_long" ]; then
    printf 'These files are longer than %s lines; split them by concern:\n%s\n' \
        "$limit" "$too_long" >&2
    exit 1
fi
echo "every Rust file is at most $limit lines"
