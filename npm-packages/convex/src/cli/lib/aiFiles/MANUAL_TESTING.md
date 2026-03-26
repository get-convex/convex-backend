# Manual Testing Guide - Convex AI Files

Most smoke scenarios are now covered by automated tests:

- unit tests in `config.test.ts` and `index.test.ts`
- temp-dir integration tests in `integration.test.ts`
- interactive prompt tests in `prompt.test.ts` (uses `@inquirer/confirm` with
  controlled I/O to verify yes/no branching in `maybeSetupAiFiles`)

The integration suite covers install, update, status, disable, enable, remove
flows, functions directory overrides, nested-directory detection, AGENTS.md /
CLAUDE.md preservation, legacy cursor rules cleanup, staleness nag logic, and
locally modified guidelines skip behavior.

Only the scenario below remains manual because it depends on real
environment/agent tooling.

---

## Manual Test 1 - Real skills installation paths across agents

**Scenario:** verify `npx skills` writes to real agent-native locations.

After `npx convex ai-files install` (or `update`) verify:

- `.agents/skills/` (canonical project-scoped install)
- `.cursor/skills/` (Cursor symlinks)
- `.claude/skills/` (Claude Code symlinks)

Expected skill set (as of Mar 2026): `convex-create-component`,
`convex-migration-helper`, `convex-performance-audit`, `convex-quickstart`,
`convex-setup-auth`.

Note: the live skill set is fetched remotely and may change. Check the current
list against what `npx convex ai-files status` reports after install.

**Why manual:** these paths and symlink behaviors are environment-dependent and
are mocked in automated tests.
