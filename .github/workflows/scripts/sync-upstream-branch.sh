#!/usr/bin/env bash
#
# Merges upstream/$UPSTREAM_BRANCH into $TARGET_BRANCH. On clean merge,
# pushes to origin and exits 0. On real (post-merge-driver) conflict,
# pushes upstream's HEAD to a sync branch, opens or comments on a PR
# from that branch back into $TARGET_BRANCH, and exits 1.
#
# Required env:
#   UPSTREAM_REPOSITORY  e.g. get-convex/convex-backend
#   TARGET_BRANCH        e.g. enhanced
#   UPSTREAM_BRANCH      e.g. main
#   GH_TOKEN             token with repo + PR write permissions
# Optional env:
#   OPENROUTER_API_KEY   if set, conflicts are first sent to an LLM for
#                        auto-resolution before falling back to the PR
#                        path. See scripts/llm-resolve-conflicts.sh.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Reconcile Cargo.lock against the workspace and amend the current
# (merge) commit if it changed. Upstream's lockfile (taken via the
# `merge=theirs` driver in .gitattributes) lacks our fork-local crates
# — orchestrator, orchestrator_api_types — so `cargo update -w --locked`
# in downstream CI would fail. Pre-emptively regenerate here so we
# never push a lockfile that's out of sync with our Cargo.toml.
reconcile_cargo_lock() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not on PATH; skipping Cargo.lock reconciliation" >&2
    return 0
  fi
  if [ ! -f Cargo.toml ]; then
    return 0
  fi
  cargo update --workspace 2>&1 | sed 's/^/  /'
  if ! git diff --quiet Cargo.lock; then
    git add Cargo.lock
    git commit --amend --no-edit
    echo "Amended merge commit with regenerated Cargo.lock"
  fi
}

# Reconcile Rush's generated pnpm-lock.yaml against the merged workspace.
# The upstream lockfile can be correct for upstream while stale for this fork's
# extra dashboard packages, so regenerate it before pushing sync commits.
reconcile_rush_shrinkwrap() {
  if ! command -v node >/dev/null 2>&1; then
    echo "node not on PATH; skipping Rush shrinkwrap reconciliation" >&2
    return 0
  fi
  if [ ! -f npm-packages/rush.json ]; then
    return 0
  fi
  (
    cd npm-packages
    node common/scripts/install-run-rush.js update
  ) 2>&1 | sed 's/^/  /'
  if ! git diff --quiet -- npm-packages/common/config/rush/pnpm-lock.yaml; then
    git add npm-packages/common/config/rush/pnpm-lock.yaml
    git commit --amend --no-edit
    echo "Amended merge commit with regenerated Rush shrinkwrap"
  fi
}

reconcile_generated_locks() {
  reconcile_cargo_lock
  reconcile_rush_shrinkwrap
}

git remote add upstream "https://github.com/${UPSTREAM_REPOSITORY}.git" 2>/dev/null || true
git fetch --no-tags origin "${TARGET_BRANCH}" || true
git fetch --no-tags upstream "${UPSTREAM_BRANCH}"

if git show-ref --verify --quiet "refs/remotes/origin/${TARGET_BRANCH}"; then
  git checkout -B "${TARGET_BRANCH}" "origin/${TARGET_BRANCH}"
else
  # Target branch doesn't exist on origin yet — bootstrap from upstream.
  git checkout -B "${TARGET_BRANCH}" "upstream/${UPSTREAM_BRANCH}"
  git push origin "HEAD:${TARGET_BRANCH}"
  exit 0
fi

sync_branch="sync/upstream-${UPSTREAM_BRANCH}-to-${TARGET_BRANCH}"
run_url="${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}"

before="$(git rev-parse HEAD)"

if git merge --no-edit "upstream/${UPSTREAM_BRANCH}"; then
  after="$(git rev-parse HEAD)"
  if [ "$before" = "$after" ]; then
    echo "${TARGET_BRANCH} is already up to date with upstream/${UPSTREAM_BRANCH}"
  else
    reconcile_generated_locks
    git push origin "HEAD:${TARGET_BRANCH}"
  fi

  # If a sync PR was previously opened, close it — we're caught up.
  existing_pr="$(gh pr list --head "${sync_branch}" --base "${TARGET_BRANCH}" --state open --json number --jq '.[0].number' 2>/dev/null || true)"
  if [ -n "${existing_pr}" ]; then
    gh pr close "${existing_pr}" --comment "Resolved — sync completed cleanly in [run](${run_url})." || true
  fi
  exit 0
fi

# Real conflicts remain even after merge drivers ran.
conflicted_files="$(git diff --name-only --diff-filter=U || true)"

if [ -z "${conflicted_files}" ]; then
  git merge --abort
  echo "Merge failed with no unmerged paths — unexpected state." >&2
  exit 1
fi

# Attempt LLM auto-resolution if OPENROUTER_API_KEY is set. The merge
# is left in progress so the script can write resolutions over the
# conflicted files; on success we stage + commit + push. On failure
# we `git merge --abort` and fall through to the PR path.
if [ -n "${OPENROUTER_API_KEY:-}" ]; then
  echo "Attempting LLM-based conflict resolution..."
  # shellcheck disable=SC2086
  if "${script_dir}/llm-resolve-conflicts.sh" ${conflicted_files}; then
    git add -- ${conflicted_files}
    # Belt-and-braces: every conflicted path must now be staged-clean.
    if git diff --name-only --diff-filter=U | grep -q .; then
      echo "LLM resolution left unmerged paths; falling back to PR." >&2
      git merge --abort
    else
      llm_model_used="${LLM_MODEL:-anthropic/claude-sonnet-4.6}"
      file_list="$(printf '%s\n' "${conflicted_files}" | sed 's|^|- |')"
      git commit --no-edit -m "Merge remote-tracking branch 'upstream/${UPSTREAM_BRANCH}' into ${TARGET_BRANCH}

LLM-auto-resolved conflicts via ${llm_model_used}:
${file_list}

Workflow run: ${run_url}"
      reconcile_generated_locks
      git push origin "HEAD:${TARGET_BRANCH}"

      # Close any stale sync PR — we just merged cleanly.
      existing_pr="$(gh pr list --head "${sync_branch}" --base "${TARGET_BRANCH}" --state open --json number --jq '.[0].number' 2>/dev/null || true)"
      if [ -n "${existing_pr}" ]; then
        gh pr close "${existing_pr}" --comment "Resolved by LLM auto-merge in [run](${run_url})." || true
      fi
      exit 0
    fi
  else
    echo "LLM resolution failed; falling back to PR path." >&2
    git merge --abort
  fi
else
  echo "OPENROUTER_API_KEY not set; skipping LLM resolution."
  git merge --abort
fi

# Force-push upstream's current HEAD to the sync branch so the PR diff
# shows exactly the commits that need to be integrated.
git push --force origin \
  "refs/remotes/upstream/${UPSTREAM_BRANCH}:refs/heads/${sync_branch}"

# Build PR body. Bash string interpolation handles the newlines.
conflict_list="$(printf '%s\n' "${conflicted_files}" | sed 's|^|- `|; s|$|`|')"
body="Sync workflow hit unresolved conflicts merging \`upstream/${UPSTREAM_BRANCH}\` into \`${TARGET_BRANCH}\`.

**Conflicted files:**
${conflict_list}

To resolve locally:
\`\`\`
git fetch origin ${TARGET_BRANCH} ${sync_branch}
git checkout ${TARGET_BRANCH}
git merge origin/${sync_branch}
# resolve, then:
git push origin ${TARGET_BRANCH}
\`\`\`

[Workflow run](${run_url})"

existing_pr="$(gh pr list --head "${sync_branch}" --base "${TARGET_BRANCH}" --state open --json number --jq '.[0].number' 2>/dev/null || true)"
if [ -n "${existing_pr}" ]; then
  gh pr comment "${existing_pr}" --body "${body}"
else
  gh pr create \
    --base "${TARGET_BRANCH}" \
    --head "${sync_branch}" \
    --title "Sync upstream/${UPSTREAM_BRANCH} → ${TARGET_BRANCH} (conflicts)" \
    --body "${body}"
fi

exit 1
