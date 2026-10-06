# Bulk Platform Convex Runtime fork

## Purpose

Bulk self-hosts Convex and uses this fork for the production backend runtime.
The fork exists so high-capacity installations can tune the two self-hosted HTTP
listeners independently while keeping the rest of Convex close to upstream.

| Concern                                    | Source of truth                    |
| ------------------------------------------ | ---------------------------------- |
| Convex Runtime Rust source                 | This repository                    |
| Convex application functions and schema    | `Bulk-Platform/bulk_v3/backend_v3` |
| Runtime image pins, limits, and deployment | `Bulk-Platform/bulk-infra`         |
| Upstream Convex source                     | `get-convex/convex-backend`        |

The Bulk-only runtime knobs are:

- `LOCAL_BACKEND_MAX_CONCURRENT_REQUESTS` for the main self-hosted listener.
- `SITE_PROXY_MAX_CONCURRENT_REQUESTS` for the public HTTP-action proxy.
- `EXPORT_MALLOC_TRIM_ENABLED` to run glibc `malloc_trim(0)` on a blocking
  worker after each successful snapshot export. It defaults to `false` and
  should only be enabled after measuring trim duration and query tail latency.
- `ALLOCATOR_MALLOC_TRIM_INTERVAL_SECS` enables a self-hosted periodic cleanup
  timer. It defaults to zero (disabled); positive values below 60 are rejected.
  The timer waits a full interval before its first pass and after each pass.
  Both triggers share a nonblocking lock, so overlapping trims are skipped. Use
  a fixed-load canary before enabling this on another host.

The fork also carries the small self-hosted export-memory fix from upstream PR
#436 while upstream issue #435 remains open. It unregisters `_file_storage`
metadata after each component export and avoids reading a whole local-storage
object merely to determine its size. Keep this patch until upstream ships an
equivalent fix.

Bulk's glibc-based self-hosted image can retain free pages across repeated
exports even after the export buffers are dropped. The opt-in trim hook records
attempts, duration, and observed RSS reclaimed as
`snapshot_export_malloc_trim_total`, `snapshot_export_malloc_trim_seconds`, and
`snapshot_export_malloc_trim_reclaimed_bytes`. Do not combine it with
jemalloc-only `MALLOC_CONF` settings or an artificial glibc arena cap.

The shared hook additionally records `allocator_malloc_trim_total` and
`allocator_malloc_trim_seconds` labeled by `reason` (`export` or `periodic`).
Logs include glibc arena allocated/free bytes and directly mapped bytes before
and after cleanup. These are allocator statistics, not an exact accounting of
resident pages; thread caches are included in the allocated estimate. Existing
export metric names remain available. No database format changes are involved.

The implementations live in `crates/common/src/knobs.rs` and
`crates/local_backend/src/`. Production values belong in `bulk-infra`, not in
this repository.

## Patch policy

Keep Bulk-only changes narrow, tested, and separable from upstream code. Before
adding a patch, prefer an upstream contribution when it is generally useful.
Every retained patch must have:

- a clear production reason;
- a test or build check at the changed seam;
- an entry in this document when it changes the supported runtime contract;
- a compatible `bulk-infra` change when deployment settings must change.

## Sync with upstream

Add the upstream remote once:

```bash
git remote add upstream https://github.com/get-convex/convex-backend.git
git fetch origin --prune
git fetch upstream --prune
```

Prepare an upgrade without rewriting shared history:

```bash
git switch -c codex/upgrade-convex-YYYYMMDD origin/main
git merge --no-ff upstream/main
git log --oneline upstream/main..HEAD
git diff --stat upstream/main...HEAD
```

Resolve conflicts by preserving current upstream behavior first, then replaying
the small Bulk patch set. Do not force-push `main`. Open a pull request and make
the upstream commit or release being adopted clear in its description.

Minimum local validation for runtime changes:

```bash
cargo fmt --all --check
cargo check -p local_backend
cargo test -p local_backend
```

If a full local test is impractical, explain that in the pull request and rely
on the Linux image build plus its published-binary smoke test before deployment.

## Build and publish

Bulk's build toolchain and self-hosted Node action executor use Node.js 24.21.0,
pinned in `.nvmrc`. The backend Dockerfile installs that exact NodeSource version
and copies the binary into the final runtime image. The release smoke test checks
the final image's Node version against `.nvmrc`, exercises crypto, and verifies
the fetch API before reporting an immutable digest. Upstream's Node executor
already accepts Node 24; this change does not upgrade the Rust/V8 query runtime
or change application schema and function contracts.

For an isolated dev-slot test before merging, run the GitHub Actions workflow
from a `codex/upgrade-convex-*` branch:

```bash
gh workflow run bulk_release_backend.yml \
  --repo Bulk-Platform/convex-backend \
  --ref codex/upgrade-convex-YYYYMMDD
```

The upgrade-branch path still publishes only the commit SHA tag and immutable
digest. It does not update `latest` or any deployment. Pin that digest only on
the selected disposable dev slot.

After the fork pull request is merged to `main`, run the same workflow on
`main`:

```bash
gh workflow run bulk_release_backend.yml \
  --repo Bulk-Platform/convex-backend \
  --ref main
gh run list \
  --repo Bulk-Platform/convex-backend \
  --workflow bulk_release_backend.yml \
  --limit 1
```

The workflow builds Linux AMD64, pushes
`ghcr.io/bulk-platform/convex-backend:<commit-sha>`, smoke-tests the published
binary, and prints the immutable `image@sha256:...` reference in its summary.

Do not deploy the commit tag or `latest`. Copy the immutable digest into a
separate `bulk-infra` pull request and follow `runbooks/convex-runtime-fork.md`
there.

## Deployment boundary

Running `./bulk` from `bulk_v3` deploys Convex application functions,
migrations, search initialization, and application version metadata. It does not
build or replace this runtime image.

A runtime release is complete only after:

1. the fork change is merged;
2. the release workflow publishes and smoke-tests an image;
3. `bulk-infra` pins the new digest and passes its image/concurrency guards;
4. the runtime is deployed through the documented canary sequence;
5. backend, site, sync, and application smoke checks pass.

## Rollback and database safety

Convex may apply internal database migrations while starting a newer runtime. Do
not assume that an older image can read a database after a forward upgrade.
Before a runtime upgrade, confirm a recent Convex backup and a tested restore
path.

If the new runtime is unhealthy, stop the rollout. Re-pin the previous known
good digest only when its database compatibility is understood. If it is not,
restore through the `bulk-infra` Convex restore runbook instead of repeatedly
restarting or downgrading the container.
