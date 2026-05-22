import {
  DEFAULT_INFRASTRUCTURE,
  infrastructureOverrides,
  type BackendInfrastructureDraft,
} from "./backendInfrastructure";

describe("infrastructureOverrides", () => {
  test("default sidecar stack persists database and storage mode markers", () => {
    const result = infrastructureOverrides(DEFAULT_INFRASTRUCTURE);

    expect(result.provisioningMode).toBe("sidecar");
    expect(result.overrides).toEqual({
      CONVEX_ORCHESTRATOR_DATABASE_MODE: "sidecar",
      CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
      CONVEX_ORCHESTRATOR_STORAGE_MODE: "sidecar",
    });
  });

  test("sqlite and local storage force volume mode", () => {
    const result = infrastructureOverrides({
      database: { kind: "sqlite" },
      storage: { kind: "local" },
    });

    expect(result.provisioningMode).toBe("volume-sqlite");
    expect(result.overrides).toEqual({
      CONVEX_ORCHESTRATOR_DATABASE_MODE: "sqlite",
      CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "volume-sqlite",
      CONVEX_ORCHESTRATOR_STORAGE_MODE: "local",
    });
  });

  test("mysql with minio uses MYSQL_URL without postgres sidecar env", () => {
    const draft: BackendInfrastructureDraft = {
      ...DEFAULT_INFRASTRUCTURE,
      database: {
        kind: "mysql",
        url: "mysql://user:pass@mysql:3306",
      },
    };

    expect(infrastructureOverrides(draft)).toEqual({
      provisioningMode: "sidecar",
      overrides: {
        CONVEX_ORCHESTRATOR_DATABASE_MODE: "external",
        CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
        CONVEX_ORCHESTRATOR_STORAGE_MODE: "sidecar",
        MYSQL_URL: "mysql://user:pass@mysql:3306",
      },
    });
  });

  test("mysql with local storage avoids sidecars entirely", () => {
    const draft: BackendInfrastructureDraft = {
      database: {
        kind: "mysql",
        url: " mysql://user:pass@mysql:3306 ",
      },
      storage: { kind: "local" },
    };

    expect(infrastructureOverrides(draft)).toEqual({
      provisioningMode: "volume-sqlite",
      overrides: {
        CONVEX_ORCHESTRATOR_DATABASE_MODE: "external",
        CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "volume-sqlite",
        CONVEX_ORCHESTRATOR_STORAGE_MODE: "local",
        MYSQL_URL: "mysql://user:pass@mysql:3306",
      },
    });
  });

  test("s3 compatible storage includes endpoint and compatibility flags", () => {
    const draft: BackendInfrastructureDraft = {
      ...DEFAULT_INFRASTRUCTURE,
      storage: {
        kind: "s3-compatible",
        region: "us-east-1",
        endpointUrl: "https://storage.example.com",
        accessKeyId: "key",
        secretAccessKey: "secret",
        forcePathStyle: true,
        disableSse: true,
        disableChecksums: true,
        buckets: {
          exports: "exports",
          snapshotImports: "imports",
          modules: "modules",
          files: "files",
          search: "search",
        },
      },
    };

    expect(infrastructureOverrides(draft)).toEqual({
      provisioningMode: "sidecar",
      overrides: {
        AWS_ACCESS_KEY_ID: "key",
        AWS_REGION: "us-east-1",
        AWS_S3_DISABLE_CHECKSUMS: "true",
        AWS_S3_DISABLE_SSE: "true",
        AWS_S3_FORCE_PATH_STYLE: "true",
        AWS_SECRET_ACCESS_KEY: "secret",
        CONVEX_ORCHESTRATOR_DATABASE_MODE: "sidecar",
        CONVEX_ORCHESTRATOR_PROVISIONING_MODE: "sidecar",
        CONVEX_ORCHESTRATOR_STORAGE_MODE: "external",
        S3_ENDPOINT_URL: "https://storage.example.com",
        S3_STORAGE_EXPORTS_BUCKET: "exports",
        S3_STORAGE_FILES_BUCKET: "files",
        S3_STORAGE_MODULES_BUCKET: "modules",
        S3_STORAGE_SEARCH_BUCKET: "search",
        S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET: "imports",
      },
    });
  });
});
