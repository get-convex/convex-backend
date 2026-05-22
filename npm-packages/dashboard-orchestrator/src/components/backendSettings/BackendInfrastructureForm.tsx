import { TextInput } from "@ui/TextInput";
import {
  DEFAULT_BUCKETS,
  type BackendInfrastructureDraft,
  type DatabaseDraft,
  type ObjectStorageDraft,
} from "./backendInfrastructure";

export function BackendInfrastructureForm({
  value,
  onChange,
}: {
  value: BackendInfrastructureDraft;
  onChange: (next: BackendInfrastructureDraft) => void;
}) {
  const setDatabase = (database: DatabaseDraft) =>
    onChange({ ...value, database });
  const setStorage = (storage: ObjectStorageDraft) =>
    onChange({ ...value, storage });

  return (
    <div className="flex flex-col gap-3">
      <div className="grid gap-3 md:grid-cols-2">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-content-primary">Database</span>
          <select
            value={value.database.kind}
            onChange={(e) => {
              const kind = e.target.value as DatabaseDraft["kind"];
              if (kind === "postgres") {
                setDatabase({ kind, url: "" });
              } else if (kind === "mysql") {
                setDatabase({ kind, url: "" });
              } else {
                setDatabase({ kind });
              }
            }}
            className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
          >
            <option value="sidecar">Postgres sidecar</option>
            <option value="sqlite">SQLite volume</option>
            <option value="postgres">External Postgres</option>
            <option value="mysql">External MySQL</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-content-primary">
            Object storage
          </span>
          <select
            value={value.storage.kind}
            onChange={(e) => {
              const kind = e.target.value as ObjectStorageDraft["kind"];
              if (kind === "s3" || kind === "s3-compatible") {
                setStorage({
                  kind,
                  region: "us-east-1",
                  accessKeyId: "",
                  secretAccessKey: "",
                  endpointUrl: "",
                  forcePathStyle: kind === "s3-compatible",
                  disableSse: kind === "s3-compatible",
                  disableChecksums: kind === "s3-compatible",
                  buckets: { ...DEFAULT_BUCKETS },
                });
              } else {
                setStorage({ kind });
              }
            }}
            className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
          >
            <option value="sidecar">MinIO sidecar</option>
            <option value="local">Local volume</option>
            <option value="s3">AWS S3</option>
            <option value="s3-compatible">S3 compatible</option>
          </select>
        </label>
      </div>
      {(value.database.kind === "postgres" ||
        value.database.kind === "mysql") && (
        <TextInput
          id="backendDatabaseUrl"
          label={
            value.database.kind === "postgres" ? "Postgres URL" : "MySQL URL"
          }
          value={value.database.url}
          onChange={(e) =>
            setDatabase(
              value.database.kind === "postgres"
                ? { kind: "postgres", url: e.target.value }
                : { kind: "mysql", url: e.target.value },
            )
          }
          placeholder={
            value.database.kind === "postgres"
              ? "postgres://user:pass@host:5432"
              : "mysql://user:pass@host:3306"
          }
        />
      )}
      {(value.storage.kind === "s3" ||
        value.storage.kind === "s3-compatible") && (
        <S3Fields
          value={value.storage}
          onChange={(storage) => setStorage(storage)}
        />
      )}
    </div>
  );
}

function S3Fields({
  value,
  onChange,
}: {
  value: Extract<ObjectStorageDraft, { kind: "s3" | "s3-compatible" }>;
  onChange: (
    next: Extract<ObjectStorageDraft, { kind: "s3" | "s3-compatible" }>,
  ) => void;
}) {
  const setBucket = (key: keyof typeof value.buckets, next: string) =>
    onChange({ ...value, buckets: { ...value.buckets, [key]: next } });

  return (
    <div className="flex flex-col gap-3">
      <div className="grid gap-3 md:grid-cols-2">
        <TextInput
          id="backendS3Region"
          label="Region"
          value={value.region}
          onChange={(e) => onChange({ ...value, region: e.target.value })}
          placeholder="us-east-1"
        />
        <TextInput
          id="backendS3Endpoint"
          label="Endpoint URL"
          value={value.endpointUrl ?? ""}
          onChange={(e) => onChange({ ...value, endpointUrl: e.target.value })}
          placeholder="https://s3.amazonaws.com"
        />
        <TextInput
          id="backendS3AccessKey"
          label="Access key ID"
          value={value.accessKeyId}
          onChange={(e) => onChange({ ...value, accessKeyId: e.target.value })}
        />
        <TextInput
          id="backendS3SecretKey"
          label="Secret access key"
          value={value.secretAccessKey}
          onChange={(e) =>
            onChange({ ...value, secretAccessKey: e.target.value })
          }
          type="password"
        />
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <BucketInput
          id="backendS3ExportsBucket"
          label="Exports bucket"
          value={value.buckets.exports}
          onChange={(next) => setBucket("exports", next)}
        />
        <BucketInput
          id="backendS3SnapshotImportsBucket"
          label="Snapshot imports bucket"
          value={value.buckets.snapshotImports}
          onChange={(next) => setBucket("snapshotImports", next)}
        />
        <BucketInput
          id="backendS3ModulesBucket"
          label="Modules bucket"
          value={value.buckets.modules}
          onChange={(next) => setBucket("modules", next)}
        />
        <BucketInput
          id="backendS3FilesBucket"
          label="Files bucket"
          value={value.buckets.files}
          onChange={(next) => setBucket("files", next)}
        />
        <BucketInput
          id="backendS3SearchBucket"
          label="Search bucket"
          value={value.buckets.search}
          onChange={(next) => setBucket("search", next)}
        />
      </div>
      <div className="flex flex-wrap gap-3 text-sm text-content-primary">
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={!!value.forcePathStyle}
            onChange={(e) =>
              onChange({ ...value, forcePathStyle: e.target.checked })
            }
          />
          Path-style URLs
        </label>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={!!value.disableSse}
            onChange={(e) =>
              onChange({ ...value, disableSse: e.target.checked })
            }
          />
          Disable SSE
        </label>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={!!value.disableChecksums}
            onChange={(e) =>
              onChange({ ...value, disableChecksums: e.target.checked })
            }
          />
          Disable checksums
        </label>
      </div>
    </div>
  );
}

function BucketInput({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (next: string) => void;
}) {
  return (
    <TextInput
      id={id}
      label={label}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
