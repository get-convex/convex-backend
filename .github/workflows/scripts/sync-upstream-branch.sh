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

set -euo pipefail

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
    git push origin "HEAD:${TARGET_BRANCH}"
  fi

  # If a sync PR was previously opened, close it — we're caught up.
  existing_pr="$(gh pr list --head "${sync_branch}" --base "${TARGET_BRANCH}" --state open --json number --jq '.[0].number' 2>/dev/null || true)"
  if [ -n "${existing_pr}" ]; then
    gh pr close "${existing_pr}" --comment "Resolved — sync completed cleanly in [run](${run_url})." || true
  fi
  exit 0
fi

# Real conflicts remain even after merge drivers ran. Open a PR.
conflicted_files="$(git diff --name-only --diff-filter=U || true)"
git merge --abort

if [ -z "${conflicted_files}" ]; then
  echo "Merge failed with no unmerged paths — unexpected state." >&2
  exit 1
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
