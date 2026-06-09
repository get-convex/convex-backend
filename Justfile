set fallback := true
set shell := ["bash", "-uc"]
set windows-shell := ["sh", "-uc"]

instance_name := "open-source-convex-backend"
storage_dir := justfile_directory() / "convex_local_storage"
instance_secret_file := storage_dir / "oss_instance_secret"
admin_key_file := storage_dir / "oss_admin_key"

# `just --list` (or just `just`) will print all the recipes in
# the current Justfile. `just RECIPE` will run the macro/job.
#
# In several places there are recipes for running common scripts or commands.
# Instead of `Makefile`s, Convex uses Justfiles, which are similar, but avoid
# several footguns associated with Makefiles, since using make as a macro runner
# can sometimes conflict with Makefiles desire to have some rudimentary
# understanding of build artifacts and associated dependencies.
#
# Read up on just here: https://github.com/casey/just

_default:
  @just --list

set positional-arguments

# (*) Run the open source convex backend on port 3210
run-local-backend *ARGS:
  cargo run -p local_backend --bin convex-local-backend -- \
    --instance-name {{instance_name}} \
    --instance-secret "$(just init-instance-secret)" \
    "$@"

run-dashboard *ARGS:
  cd '{{justfile_directory() / "npm-packages/dashboard-self-hosted"}}' && \
    if [ -n "{{ARGS}}" ]; then \
      NEXT_PUBLIC_DEPLOYMENT_URL="{{ARGS}}" npm run dev; \
    else \
      NEXT_PUBLIC_DEPLOYMENT_URL="http://127.0.0.1:3210" \
      NEXT_PUBLIC_ADMIN_KEY="$(just generate-admin-key)" \
      npm run dev; \
    fi

# Uses an admin key from admin_key.txt for dev backends.
# This uses the default admin key for local backends, which is safe as long as the backend is
# running locally.
# (*) Run convex CLI commands like `convex dev` against local backend from `just run-local-backend`.
convex *ARGS:
  cd {{invocation_directory()}}; npx convex "$@" --admin-key "$(just generate-admin-key)" --url "http://127.0.0.1:3210"

# Clears any data or stored files from the local backend.
reset-local-backend:
  rm -rf convex_local_storage && rm -f convex_local_backend.sqlite3

# Global JavaScript tools
# Common commands are
# - `just rush build` to build all projects in npm-packages
# - `just rush rebuild` to build when rush doesn't realize something's changed
# - `just rush install` when the repo has changed JS deps
# - `just rush update` when you're changing JS deps
# (*) rush, the monorepo JS tool for deps and building
rush *ARGS:
  cd {{invocation_directory()}}; "{{justfile_directory()}}/scripts/rush_from_npm-packages.sh" "$@"

# Generates (or reuses) a random per-installation instance secret
init-instance-secret:
  @mkdir -p '{{storage_dir}}'
  @[ -s '{{instance_secret_file}}' ] || openssl rand -hex 32 > '{{instance_secret_file}}'
  @cat '{{instance_secret_file}}'

# Generates (or reuses) an admin key derived from the local instance secret
generate-admin-key:
  @just init-instance-secret > /dev/null
  @[ -s '{{admin_key_file}}' ] || cargo run --quiet --bin generate_key -- {{instance_name}} "$(cat '{{instance_secret_file}}')" 2>/dev/null > '{{admin_key_file}}'
  @cat '{{admin_key_file}}'
