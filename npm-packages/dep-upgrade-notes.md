# npm dependency notes

Rush pushes us toward using a single a version of each dependency in the
monorepo. Our version choices are compromises between the competing needs of

- staying up to date enough to cutting edge to be able to pull in security
  updates
- staying up to date because we like nice things
- shipping very compatible client code
- using old AND new versions in demos and tests that are representative of
  customer use cases.

We can use multiple versions by adding the less common one to
allowedAlternativeVersions in
npm-packages/common/config/rush/common-versions.json

# Rush

We don't really need Rush, we mostly use features of pnpm. We could replace Rush
with Turborepo if we wanted to.

See rush.json for notes on what version we use.

# node version

We've dropped 16, we need to support 18 for a while. Some things are starting to
require 20 so it would be nice to upgrade to 20 by default, just using 18 for a
few integration tests.

Currently function runner / local backend expect Node.js 18 to be the default in
order to simulate our Lambda environment. Devs probably have to be able to
choose their Node.js environment (18, 20, or 22) before we can easily upgrade
past v18.

The local backend failing when run with another version of Node.js might be a
blocker for local dev.

# Dependencies that are hard to upgrade

Run `just rush update --full` to upgrade withing semver specs. If this doesn't
work we need to narrow our semver requirement spec for that library.

Run `just rush upgrade-interactive` to upgrade libraries beyond their current
semver spec. See notes below for these libraries.

### react and react-dom

We may need to support React 17 for a long time because React 18 includes
significant implementation changes.

We can upgrade to 18, but we should create a test project that uses 17.

React 19 has been released. For types, keep using React 18 to ensure our code is
compatible. Clerk does something like this.

### TypeScript

We should be more aggressive here. Currently we require 5.0.3 as a minimum. We
can go up from here.

### jwt-decode

This is a unbundled dep, we had issues when we upgraded.
https://github.com/get-convex/convex/pull/30674

### Chalk

We want to stay on version 4 of chalk forever (well until ship the CLI as CJS,
version 5 is an esm-only build but we want to need to bundle CJS into our CLI.

### openid-client

It's a rewrite, will need to test carefully.

### Inquirer

Just haven't gotten to it, breaking changes are probably simple

### Commander

dunno

### Vitest 1->2

Big changes

### esbuild

macOS 10.15 Catalina no longer supported in esbuild@0.24.0, maybe we care

### typedoc

A bit of custom work to do, we have patched versions of some libraries in docs
that depend on this. Maybe this means we can get rid of them!

### sentry/node 8

esm something or other

### abortcontroller-polyfill

1.7.7
(https://github.com/mo/abortcontroller-polyfill/commit/575383ecb91a0f77a571b59e9c4e223832f032d9
maybe?) breaks something
