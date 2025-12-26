## Using S3 Storage

By default, the backend stores file data on the filesystem within the docker
container. To instead run the backend with S3 storage, set up the following
buckets and environment variables.

```sh
export AWS_REGION="your-region"
export AWS_ACCESS_KEY_ID="your-access-key-id"
export AWS_SECRET_ACCESS_KEY="your-secret-access-key"
export S3_STORAGE_EXPORTS_BUCKET="convex-snapshot-exports"
export S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET="convex-snapshot-imports"
export S3_STORAGE_MODULES_BUCKET="convex-modules"
export S3_STORAGE_FILES_BUCKET="convex-user-files"
export S3_STORAGE_SEARCH_BUCKET="convex-search-indexes"
```

Optionally set the `S3_ENDPOINT_URL` environment variable. This is required for
using [R2](https://www.cloudflare.com/developer-platform/products/r2/) or some
other drop-in replacement compatible with the AWS S3 API.

Then run the backend!

## Migrating storage providers

If you are switching between local storage and S3 storage (or vice versa),
you'll need to run a snapshot export and import to migrate your data.

Run:

```sh
npx convex export --path <path-to-export-file>
```

Then set up a fresh backend with the new storage provider and import the data:

```sh
npx convex import --replace-all <path-to-export-file>
```
