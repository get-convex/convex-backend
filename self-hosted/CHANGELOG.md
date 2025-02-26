# Changelog

Find the latest versions of the `convex-backend` package
[here](https://github.com/get-convex/convex-backend/pkgs/container/convex-backend)
and `convex-dashboard` package
[here](https://github.com/get-convex/convex-backend/pkgs/container/convex-dashboard).
Make sure to use the same version of `convex-backend` and `convex-dashboard`.
Different versions are not guaranteed to be compatible with one another.

Follow the instructions in the [README](README.md#software-upgrades) to upgrade
your self-hosted backend and dashboard.

## 2025-02-26 `161e32648a971fb8ef591e61212f7b9fb7ff4f2c`

- Add support for S3 storage for exports, snapshots, modules, files, and search
  indexes. Read more on how to set up S3 storage
  [here](README.md#using-s3-storage).

## 2025-02-24 `fff8e431b95f4d9fde899ce348f8e8f23210aad3`

- Support streaming import. Read more on how to set up streaming import
  [here](https://docs.convex.dev/production/integrations/streaming-import-export#streaming-import)
- Fix bug in routing to HTTP actions. Read more
  [here](https://github.com/get-convex/convex-backend/commit/1652ee81d8a01fdeed98b0e4c923a89d1672f8ad).

## 2025-02-19 `86ae5d34c8164075b66fa0c52beabd19212d8df7`

- Fix bug in MySQL where certificates were not verified upon connection. Now,
  you must set `DO_NOT_REQUIRE_SSL` for running locally.

## 2025-02-19 `663640f5a01018914dc4314145f23a31f3afdca6`

- Add support for MySQL! The `DATABASE_URL` env variable is now `POSTGRES_URL`
  or `MYSQL_URL`. Known issue: MySQL certificates are not verified upon
  connection. The next release will include a fix.
- Optimize database queries, so simple mutations get 4x faster when running
  against a Postgres or MySQL db in a different datacenter or region.

## 2025-02-18 `62ef09aa604b0c5f873b59e0944b5e89f84b66b2`

- Add support for running Docker image with local Postgres using the
  `DO_NOT_REQUIRE_SSL` environment variable.

## 2025-02-13 `6c974d219776b753cd23d26f4a296629ff7c2cad`

- Fix a bug where every node action request would write to temporary files that
  were never cleaned up. Caused disk space to fill up on volumes.

## 2025-02-12 `4499dd4fd7f2148687a7774599c613d052950f46`

> ⚠️ **WARNING**: DO NOT use this version in production! This initial release
> contains a critical bug that fills up disk space. Use version
> [6c974d219776b753cd23d26f4a296629ff7c2cad](##6c974d219776b753cd23d26f4a296629ff7c2cad)
> or later.

- Initial release of self-hosted backend and dashboard.
