# Running Convex in a DevContainer for Local Development

If you're working with Convex and want to use a consistent, container-based development environment, this guide provides a minimal setup using [DevContainers](https://containers.dev/) and Docker.

> [!IMPORTANT]  
> This approach is meant for **local development** and is not intended for self-hosting Convex in production.

## What is a DevContainer?

A DevContainer is a development environment defined as code and backed by a Docker container. It integrates tightly with Visual Studio Code through the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers).

When you open a project with a `.devcontainer/devcontainer.json` file, VS Code automatically builds the container, installs dependencies, and mounts your project directory inside it.

This setup is especially useful for teams, open source contributors, or anyone who wants to avoid dependency drift between local machines.

## Why use a DevContainer?

- Reproducible local environment with no host machine setup required
- Isolated from other projects and host system
- Preconfigured runtimes, dependencies, tools and extensions (e.g., Node.js, pnpm, Convex CLI)
- Easy onboarding for new team members or contributors

## Requirements

To use a DevContainer, you need to have the following installed:

- [Docker](https://www.docker.com/products/docker-desktop)
- [Visual Studio Code](https://code.visualstudio.com/)
- [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

## Minimal DevContainer Example for Convex

The following is a minimal example of a working `.devcontainer/devcontainer.json` setup using a Node.js/TypeScript base image. It binds the necessary Convex and pnpm directories, and explicitly forwards the required ports:

```jsonc
{
  "name": "convex-dev",
  "image": "mcr.microsoft.com/devcontainers/typescript-node:1-22-bookworm",
  "workspaceFolder": "/workspaces/${localWorkspaceFolderBasename}",

  "postCreateCommand": "npm install -g convex && npx convex dev --once",
  "postAttachCommand": "git config --global diff.ool ...",
  "postStartCommand": "git config --global --add safe.directory /workspaces/${localWorkspaceFolderBasename}",

  "mounts": [
    "source=${localEnv:HOME}/.ssh,target=/home/node/.ssh,type=bind,consistency=cached",
    "source=${localEnv:HOME}/.convex,target=/home/node/.convex,type=bind,consistency=cached",
    "source=${localEnv:HOME}/.cache/convex,target=/home/node/.cache,type=bind,consistency=cached"
  ],

  "remoteUser": "node",
  "forwardPorts": [3210, 6790, 6791]
}
```

You can adapt the image, remote user, or mounted paths depending on your project needs or base OS image.

### Explanation of the Configuration

This minimal setup includes just a few customizations that are important for Convex to run reliably inside a containerized environment.

#### `.convex` mount

```json
"mounts": [
  "source=${localEnv:HOME}/.convex,target=/home/node/.convex,type=bind"
]
```

Convex stores some local state in the `.convex` directory (such as deployment metadata and generated admin keys). Mounting it from your host machine into the container ensures that:

- The state is preserved across container rebuilds.
- You can reuse the same identity and credentials inside and outside the container.

Without this mount, Convex might behave as if it's being run for the first time every time you restart the container.

#### `.cache/convex` mount

```json
"source=${localEnv:HOME}/.cache/convex,target=/home/node/.cache,type=bind,consistency=cached"
```

During `pnpm convex dev`, the Convex CLI downloads necessary artifacts such as backend binaries and the dashboard frontend into the `.cache/convex` directory. By mounting this directory from the host into the container, those files are persisted between container rebuilds and restarts.

This avoids re-downloading the same artifacts every time the container is recreated, which speeds up startup and reduces bandwidth usage.

#### Forwarded ports

```json
"forwardPorts": [3210, 6790, 6791]
```

Convex uses these ports during local development:

- `3210` — the API server
- `6790` — the web dashboard
- `6791` — the internal health check used by the dashboard to determine if a local deployment is available

Forwarding these ports ensures that the services running inside the container are accessible from your host machine and from the dashboard itself.

#### `postCreateCommand`

```json
"postCreateCommand": "npx convex dev --once"
```

This command ensures the Convex development server is started as soon as the container is ready. The `--once` flag runs the server in one-off mode, avoiding watch mode or automatic restarts.

This is useful for initial setup to verify everything is working, but you can always stop it and run `pnpm convex dev` manually when actively working on your functions.
