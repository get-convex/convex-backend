#!/usr/bin/env bash
#
# Attempts to resolve every passed-in conflicted file via an OpenRouter LLM.
# Each file is sent whole; the model is asked to return the resolved file
# verbatim. Output is validated (no remaining `<<<<<<<` / `=======` /
# `>>>>>>>` markers). Files are written in place.
#
# Exit 0 only if EVERY file was resolved cleanly. Exit 1 if any file
# failed — callers should then fall back to the PR path. Files written
# before a later failure are left modified; callers are expected to be
# inside a `git merge` so `git merge --abort` cleans up.
#
# Required env:
#   OPENROUTER_API_KEY
# Optional env:
#   LLM_MODEL          default anthropic/claude-sonnet-4.6
#   LLM_MAX_FILE_BYTES default 200000

set -euo pipefail

: "${OPENROUTER_API_KEY:?OPENROUTER_API_KEY is required}"
MODEL="${LLM_MODEL:-anthropic/claude-sonnet-4.6}"
MAX_BYTES="${LLM_MAX_FILE_BYTES:-200000}"

if [ "$#" -eq 0 ]; then
  echo "no files passed" >&2
  exit 1
fi

resolve_one() {
  local file="$1"
  local size content prompt request_json response resolved

  if [ ! -f "$file" ]; then
    echo "  ✗ $file: not a regular file (delete/modify conflict?)" >&2
    return 1
  fi

  size=$(wc -c < "$file")
  if [ "$size" -gt "$MAX_BYTES" ]; then
    echo "  ✗ $file: ${size} bytes exceeds LLM_MAX_FILE_BYTES=${MAX_BYTES}" >&2
    return 1
  fi

  content="$(cat "$file")"

  prompt="You are resolving a Git merge conflict in a fork that periodically syncs from an upstream repository. The fork carries intentional divergences from upstream.

The file contents below contain conflict markers (<<<<<<< HEAD, =======, >>>>>>> upstream/...). Resolve them and output the entire resolved file.

Rules:
1. Preserve the fork's intentional changes (HEAD side) when both sides represent independent intent. Upstream-only refactors (renames, restructures) that overlap with fork changes should be APPLIED to the fork's code, not used to revert it.
2. When both sides add different things to the same region, include both if compatible.
3. Output ONLY the resolved file content. No explanations, no markdown code fences, no commentary, no preamble.
4. Do not modify any code outside the conflicted regions.
5. The file must compile/parse — preserve syntactic validity for its language.

File path: ${file}

--- BEGIN FILE ---
${content}
--- END FILE ---"

  request_json="$(jq -n \
    --arg model "$MODEL" \
    --arg prompt "$prompt" \
    '{
      model: $model,
      messages: [{role: "user", content: $prompt}],
      temperature: 0
    }')"

  if ! response="$(curl -sS --fail-with-body --max-time 180 \
    https://openrouter.ai/api/v1/chat/completions \
    -H "Authorization: Bearer ${OPENROUTER_API_KEY}" \
    -H "Content-Type: application/json" \
    -H "HTTP-Referer: https://github.com/${GITHUB_REPOSITORY:-defy-works/convex-backend}" \
    -H "X-Title: convex-backend upstream sync" \
    -d "$request_json")"; then
    echo "  ✗ $file: OpenRouter call failed" >&2
    return 1
  fi

  resolved="$(printf '%s' "$response" | jq -r '.choices[0].message.content // empty')"
  if [ -z "$resolved" ]; then
    echo "  ✗ $file: empty LLM response" >&2
    printf '%s\n' "$response" | head -c 500 >&2
    echo >&2
    return 1
  fi

  # Strip a single pair of leading/trailing markdown code fences if the
  # model added them despite instructions. Match ``` optionally followed
  # by a language tag on the first line, and ``` alone on the last.
  resolved="$(printf '%s' "$resolved" | awk '
    NR == 1 && $0 ~ /^```[a-zA-Z0-9_+-]*$/ { skip_first = 1; next }
    { lines[++n] = $0 }
    END {
      end = n
      if (skip_first && lines[end] ~ /^```$/) end--
      for (i = 1; i <= end; i++) print lines[i]
    }
  ')"

  # Validate: no remaining conflict markers anywhere in the output.
  if printf '%s\n' "$resolved" | grep -qE '^(<<<<<<<|=======|>>>>>>>)' ; then
    echo "  ✗ $file: LLM left conflict markers in output" >&2
    return 1
  fi

  printf '%s\n' "$resolved" > "$file"
  echo "  ✓ $file"
}

echo "Resolving $# file(s) via ${MODEL}..."

failed=()
for f in "$@"; do
  if ! resolve_one "$f"; then
    failed+=("$f")
  fi
done

if [ "${#failed[@]}" -gt 0 ]; then
  echo "LLM resolution failed for: ${failed[*]}" >&2
  exit 1
fi

echo "All files resolved."
