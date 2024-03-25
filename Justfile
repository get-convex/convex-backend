set fallback := true
set shell := ["bash", "-uc"]
set windows-shell := ["sh", "-uc"]

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
  cargo run -p local_backend --bin convex-local-backend -- "$@"

# Uses an admin key from admin_key.txt for dev backends.
# This uses the default admin key for local backends, which is safe as long as the backend is
# running locally.
# (*) Run convex CLI commands like `convex dev` against local backend from `just run-local-backend`.
convex *ARGS:
  cd {{invocation_directory()}}; npx convex "$@" --admin-key 0135d8598650f8f5cb0f30c34ec2e2bb62793bc28717c8eb6fb577996d50be5f4281b59181095065c5d0f86a2c31ddbe9b597ec62b47ded69782cd --url "http://127.0.0.1:3210"

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
