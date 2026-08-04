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
#   GH_TOKEN             token that can push and open PRs. Must NOT be the
#                        default GITHUB_TOKEN: repos with "Allow GitHub
#                        Actions to create and approve pull requests"
#                        disabled reject `gh pr create` from it with
#                        "Resource not accessible by integration", which
#                        silently disables the only escape hatch this
#                        script has. Pass the same PAT used for checkout.

set -euo pipefail

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

# Reconcile the generated pnpm-lock.yaml against the merged workspace. The
# upstream lockfile can be correct for upstream while stale for this fork's
# extra workspace packages (dashboard-orchestrator), so regenerate it before
# pushing sync commits. `pnpm install` (not `--frozen-lockfile`) is what
# updates the lockfile — that's `just update-js`.
reconcile_pnpm_lock() {
  if [ ! -f npm-packages/pnpm-workspace.yaml ]; then
    return 0
  fi
  if ! command -v just >/dev/null 2>&1; then
    echo "just not on PATH; skipping pnpm lockfile reconciliation" >&2
    return 0
  fi
  just update-js 2>&1 | tail -20 | sed 's/^/  /'
  if ! git diff --quiet -- npm-packages/pnpm-lock.yaml; then
    git add npm-packages/pnpm-lock.yaml
    git commit --amend --no-edit
    echo "Amended merge commit with regenerated pnpm lockfile"
  fi
}

reconcile_generated_locks() {
  reconcile_cargo_lock
  reconcile_pnpm_lock
}

# dprint pinned to the same version CI's Prettier job uses (scripts/package.json).
ensure_dprint() {
  if [ -x scripts/node_modules/.bin/dprint ]; then
    return 0
  fi
  if ! command -v npm >/dev/null 2>&1; then
    echo "npm not on PATH; cannot install dprint for validation" >&2
    return 1
  fi
  npm ci --prefix scripts 2>&1 | tail -5 | sed 's/^/  /'
  [ -x scripts/node_modules/.bin/dprint ]
}

# Repo-wide format/parse gate, run before ANY push of a merged tree. This
# guarantees a sync push can never turn the Prettier CI job red, and
# catches syntactically-invalid merge output (e.g. LLM commentary written
# into source files) no matter how it got into the tree.
validate_merged_tree() {
  echo "Validating merged tree with dprint check..."
  ensure_dprint || return 1
  if ! scripts/node_modules/.bin/dprint check 2>&1 | tail -40 | sed 's/^/  /' >&2; then
    echo "dprint check failed on the merged tree" >&2
    return 1
  fi
}

# Push upstream's HEAD to the sync branch and open (or comment on) a PR
# back into the target branch, then exit 1. $1 is a markdown reason line
# included at the top of the PR body.
bail_to_pr() {
  local reason="$1"
  git push --force origin \
    "refs/remotes/upstream/${UPSTREAM_BRANCH}:refs/heads/${sync_branch}"

  local body="${reason}

To resolve locally:
\`\`\`
git fetch origin ${TARGET_BRANCH} ${sync_branch}
git checkout ${TARGET_BRANCH}
git merge origin/${sync_branch}
# resolve, then:
git push origin ${TARGET_BRANCH}
\`\`\`

[Workflow run](${run_url})"

  local existing_pr
  existing_pr="$(gh pr list --head "${sync_branch}" --base "${TARGET_BRANCH}" --state open --json number --jq '.[0].number' 2>/dev/null || true)"
  if [ -n "${existing_pr}" ]; then
    gh pr comment "${existing_pr}" --body "${body}" && { exit 1; }
  else
    gh pr create \
      --base "${TARGET_BRANCH}" \
      --head "${sync_branch}" \
      --title "Sync upstream/${UPSTREAM_BRANCH} → ${TARGET_BRANCH} (needs human resolution)" \
      --body "${body}" && { exit 1; }
  fi

  # Reaching here means the hand-off itself failed (bad token, PR creation
  # disabled for the actor, API outage). That is how this sync once stayed
  # broken for 17 days: the merge failed, the PR was never created, and the
  # only signal was a red cron run nobody watches. Escalate to an issue,
  # which needs no PR permissions, so there is always exactly one artifact
  # a human can see.
  echo "PR hand-off failed; escalating to a GitHub issue." >&2
  local issue_title="Upstream sync into ${TARGET_BRANCH} is stuck"
  local existing_issue
  existing_issue="$(gh issue list --state open --search "${issue_title} in:title" --json number,title --jq "[.[] | select(.title == \"${issue_title}\")][0].number" 2>/dev/null || true)"
  if [ -n "${existing_issue}" ]; then
    gh issue comment "${existing_issue}" --body "${body}" || true
  else
    gh issue create --title "${issue_title}" --body "${body}" || true
  fi
  exit 1
}

# Dashboard build gate for every branch: upstream API/type changes can break
# fork-only dashboard code in files the merge never touches, which no
# format-level check can see. Release pushes additionally publish the
# self-hosted dashboard images, so this must stay fail-closed there too.
validate_dashboard_build() {
  if [ ! -f npm-packages/pnpm-workspace.yaml ]; then
    return 0
  fi
  if ! command -v just >/dev/null 2>&1; then
    echo "just not on PATH; cannot validate dashboard build" >&2
    return 1
  fi
  (
    set -e
    just install-js
    # `pkg...` builds the package and everything it depends on. --force skips
    # the turbo cache so a merged tree is always really compiled, never
    # replayed from a cache entry keyed on pre-merge inputs.
    just turbo run build --force \
      --filter=dashboard-self-hosted... \
      --filter=dashboard-orchestrator...
  ) 2>&1 | tail -40 | sed 's/^/  /'
}

# Full-workspace compile gate: `cargo check --workspace --all-targets` catches
# upstream API changes that break fork-only Rust code the merge never touched
# (e.g. a trait item rename that clean-merges but no longer compiles). Runs
# after the dashboard gate because build scripts need the Rush install, and
# first builds the JS bundles the isolate build scripts consume — the same
# set the Build Convex Backend workflow builds before cargo.
validate_backend_build() {
  if [ ! -f Cargo.toml ]; then
    return 0
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not on PATH; cannot validate backend build" >&2
    return 1
  fi
  if [ -f npm-packages/pnpm-workspace.yaml ]; then
    (
      set -e
      just install-js
      just turbo run build \
        --filter=component-tests... --filter=convex... --filter=system-udfs... \
        --filter=udf-runtime... --filter=udf-tests...
    ) 2>&1 | tail -20 | sed 's/^/  /' || return 1
  fi
  cargo check --workspace --all-targets 2>&1 | tail -60 | sed 's/^/  /'
}

# Every gate a merged tree must pass before it may be pushed, in increasing
# order of cost. Fail-closed: a gate that cannot run counts as a failure and
# the caller falls back to the human-reviewed PR path instead of pushing.
validate_merged_tree_full() {
  validate_merged_tree || return 1
  validate_dashboard_build || return 1
  validate_backend_build || return 1
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
    if ! validate_merged_tree_full; then
      git reset --hard "${before}"
      bail_to_pr "Automated sync merged \`upstream/${UPSTREAM_BRANCH}\` into \`${TARGET_BRANCH}\` cleanly, but the merged tree failed validation (dprint check / dashboard build / cargo check) — pushing it would break CI, so it was not pushed."
    fi
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

git merge --abort

# Conflicts are handed to a human, always. This used to first try an LLM
# (OpenRouter) auto-resolution pass; that was removed deliberately. It was
# expensive per sync, it repeatedly produced trees that failed the validation
# gates below, and when the provider started returning HTTP 402 it turned
# every hourly sync into a hard failure. A conflict is now simply a PR.
bail_to_pr "Sync workflow hit unresolved conflicts merging \`upstream/${UPSTREAM_BRANCH}\` into \`${TARGET_BRANCH}\`.

**Conflicted files:**
$(printf '%s\n' "${conflicted_files}" | sed 's|^|- `|; s|$|`|')"
