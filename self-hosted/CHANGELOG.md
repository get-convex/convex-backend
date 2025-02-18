# Changelog

Find the latest versions of the `convex-backend` package
[here](https://github.com/get-convex/convex-backend/pkgs/container/convex-backend)
and `convex-dashboard` package
[here](https://github.com/get-convex/convex-backend/pkgs/container/convex-dashboard).

## 62ef09aa604b0c5f873b59e0944b5e89f84b66b2

- Add support for running Docker image with local Postgres using the
  `DO_NOT_REQUIRE_SSL` environment variable.

## 6c974d219776b753cd23d26f4a296629ff7c2cad

- Fix a bug where every node action request would write to temporary files that
  were never cleaned up. Caused disk space to fill up on volumes.

## 4499dd4fd7f2148687a7774599c613d052950f46

> ⚠️ **WARNING**: DO NOT use this version in production! This initial release
> contains a critical bug that fills up disk space. Use version
> [6c974d219776b753cd23d26f4a296629ff7c2cad](##6c974d219776b753cd23d26f4a296629ff7c2cad)
> or later.

- Initial release of self-hosted backend and dashboard.
