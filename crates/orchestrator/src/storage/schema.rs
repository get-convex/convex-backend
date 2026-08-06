//! Postgres schema for the orchestrator's database.
//!
//! Wire-compatible with what the dashboard, CLI, and `crates/big_brain_client`
//! deserialize. Column types target Postgres 14+ (PlanetScale-Postgres
//! compatible). All timestamps are stored as `BIGINT` unix-milliseconds for
//! easy serialization to the JS clients.

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS members (
    id BIGSERIAL PRIMARY KEY,
    auth_user_id TEXT NOT NULL UNIQUE,
    -- Mirrored from BetterAuth's `user.email`; BetterAuth enforces email
    -- uniqueness in its own table, so we don't constrain it here.
    primary_email TEXT NOT NULL,
    name TEXT,
    creation_time BIGINT NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX IF NOT EXISTS members_auth_user_idx ON members(auth_user_id);
CREATE INDEX IF NOT EXISTS members_email_idx ON members(primary_email);

CREATE TABLE IF NOT EXISTS teams (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    creator_id BIGINT REFERENCES members(id) ON DELETE SET NULL,
    creation_time BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS team_members (
    team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    member_id BIGINT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin','developer')),
    PRIMARY KEY (team_id, member_id)
);

CREATE TABLE IF NOT EXISTS projects (
    id BIGSERIAL PRIMARY KEY,
    team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    is_demo BOOLEAN NOT NULL DEFAULT FALSE,
    creation_time BIGINT NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    tier TEXT NOT NULL DEFAULT 'S16',
    knob_overrides JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (team_id, slug)
);
CREATE INDEX IF NOT EXISTS projects_team_idx ON projects(team_id, deleted);

CREATE TABLE IF NOT EXISTS deployments (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL UNIQUE,
    deployment_type TEXT NOT NULL CHECK (deployment_type IN ('prod','dev','preview')),
    deployment_class TEXT NOT NULL DEFAULT 'standard',
    region TEXT,
    url TEXT NOT NULL,
    site_url TEXT NOT NULL,
    backend_pid BIGINT,
    backend_port BIGINT NOT NULL,
    creator_id BIGINT REFERENCES members(id) ON DELETE SET NULL,
    creation_time BIGINT NOT NULL,
    state TEXT NOT NULL DEFAULT 'running' CHECK (state IN ('running','paused','disabled','provisioning')),
    preview_identifier TEXT,
    instance_secret TEXT NOT NULL DEFAULT '',
    tier TEXT NOT NULL DEFAULT 'S16',
    knob_overrides JSONB NOT NULL DEFAULT '{}'::jsonb,
    desired_tier TEXT,
    desired_overrides JSONB NOT NULL DEFAULT '{}'::jsonb,
    storage_mode TEXT NOT NULL DEFAULT 'volume-sqlite' CHECK (storage_mode IN ('volume-sqlite','sidecar')),
    pg_password TEXT,
    minio_root_user TEXT,
    minio_root_password TEXT,
    backend_instance_secret TEXT
);
CREATE INDEX IF NOT EXISTS deployments_project_idx ON deployments(project_id, deployment_type);

CREATE TABLE IF NOT EXISTS access_tokens (
    id BIGSERIAL PRIMARY KEY,
    public_id TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('pat','team','deploy_prod','deploy_dev','deploy_preview','project_deploy','session','app','admin')),
    member_id BIGINT REFERENCES members(id) ON DELETE CASCADE,
    team_id BIGINT REFERENCES teams(id) ON DELETE CASCADE,
    project_id BIGINT REFERENCES projects(id) ON DELETE CASCADE,
    deployment_id BIGINT REFERENCES deployments(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    secret_suffix TEXT NOT NULL,
    creation_time BIGINT NOT NULL,
    expiry BIGINT,
    revoked_time BIGINT
);
CREATE INDEX IF NOT EXISTS access_tokens_kind_idx ON access_tokens(kind);
CREATE INDEX IF NOT EXISTS access_tokens_team_idx ON access_tokens(team_id);
CREATE INDEX IF NOT EXISTS access_tokens_member_idx ON access_tokens(member_id);
CREATE INDEX IF NOT EXISTS access_tokens_deployment_idx ON access_tokens(deployment_id);
CREATE INDEX IF NOT EXISTS access_tokens_secret_hash_idx ON access_tokens(secret_hash);

CREATE TABLE IF NOT EXISTS default_env_vars (
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    deployment_types JSONB NOT NULL,
    PRIMARY KEY (project_id, name)
);

CREATE TABLE IF NOT EXISTS audit_log_events (
    id BIGSERIAL PRIMARY KEY,
    team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    member_id BIGINT REFERENCES members(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    metadata JSONB NOT NULL,
    creation_time BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS audit_team_time_idx ON audit_log_events(team_id, creation_time);

CREATE TABLE IF NOT EXISTS opt_ins (
    member_id BIGINT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    accepted_at BIGINT NOT NULL,
    PRIMARY KEY (member_id, name)
);

CREATE TABLE IF NOT EXISTS custom_domains (
    id BIGSERIAL PRIMARY KEY,
    deployment_id BIGINT NOT NULL REFERENCES deployments(id) ON DELETE CASCADE,
    domain TEXT NOT NULL UNIQUE,
    cert_state TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL
);

-- DNS provider API credentials, used for the ACME DNS-01 challenge.
-- `secrets` is a sodium-secretbox sealed JSON blob (never returned to the
-- dashboard) so a database dump doesn't leak the operator's DNS API tokens.
CREATE TABLE IF NOT EXISTS dns_provider_credentials (
    id BIGSERIAL PRIMARY KEY,
    team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    secrets BYTEA NOT NULL,
    created_at BIGINT NOT NULL,
    UNIQUE (team_id, name)
);

-- One ACME account (and its private key) per directory URL. The key is
-- sealed like DNS credentials: whoever holds it can issue certificates for
-- any domain this orchestrator has validated.
CREATE TABLE IF NOT EXISTS acme_accounts (
    id BIGSERIAL PRIMARY KEY,
    directory_url TEXT NOT NULL UNIQUE,
    account_url TEXT NOT NULL,
    credentials BYTEA NOT NULL,
    created_at BIGINT NOT NULL
);

-- Issued certificates, keyed by domain. Kept in the database rather than
-- only on disk so the Traefik dynamic directory can be rebuilt from scratch
-- (new host, wiped volume) without re-running ACME and burning rate limit.
-- `renew_after` is our own policy (issued_at + 60d), not a claim about the
-- certificate's notAfter — we don't parse the cert, so recording an expiry
-- we haven't read would be inventing a fact.
CREATE TABLE IF NOT EXISTS custom_domain_certs (
    domain TEXT PRIMARY KEY,
    cert_pem TEXT NOT NULL,
    key_pem TEXT NOT NULL,
    issued_at BIGINT NOT NULL,
    renew_after BIGINT NOT NULL
);

-- Per-project admin grants. Layered on top of team_members: a member can
-- only be a project_admin if they're already on the team. A team admin
-- has implicit admin rights on every project regardless of this table —
-- check team_members.role = 'admin' before consulting this.
CREATE TABLE IF NOT EXISTS project_admins (
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    member_id BIGINT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    granted_at BIGINT NOT NULL,
    granted_by BIGINT REFERENCES members(id) ON DELETE SET NULL,
    PRIMARY KEY (project_id, member_id)
);
CREATE INDEX IF NOT EXISTS project_admins_member_idx ON project_admins(member_id);

CREATE TABLE IF NOT EXISTS invitations (
    id BIGSERIAL PRIMARY KEY,
    team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL,
    code TEXT NOT NULL UNIQUE,
    invited_by BIGINT REFERENCES members(id) ON DELETE SET NULL,
    created_at BIGINT NOT NULL,
    accepted_at BIGINT
);
CREATE INDEX IF NOT EXISTS invitations_team_idx ON invitations(team_id);
CREATE INDEX IF NOT EXISTS invitations_email_idx ON invitations(email);
"#;
