# Changelog

## 1.12.1

- Fix bug where `npx convex deploy` and `npx convex dev` would incorrectly skip
  pushing if the only change was removing files

## 1.12.0

- `npx convex env set` works with `ENV_VAR_NAME=value` syntax

## 1.11.3

- Fix bug when filling out an empty env file
- Exclude files beginning with # from convex directory entry points
- Warn when pushing with an https.ts file
- throw if argument to Query.take() is not an integer

## 1.11.2

- Fix timestamps in npm convex logs

## 1.11.1

- Allow Clerk 5 (currently in beta) in convex peerDependencies
- Fix typechecking bug on Windows caused by the Node.js patch for CVE-2024-27980
  that makes running tsc.CMD directly no longer work
- Exclude jsonl from convex directory entry points
- Add autocomplete for project selection in new project flow
- output debugBundlePath as full bundle instead of as a single file

---

Find release notes for versions before 1.11.3 on the
[Convex Blog](https://news.convex.dev/tag/releases/).
