<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo-light.svg" width="600">
  <source media="(prefers-color-scheme: light)" srcset="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
  <img alt="Convex logo" src="https://static-http.s3.amazonaws.com/logo/convex-logo.svg" width="600">
</picture>
</p>

** [Convex managed hosted product](https://www.convex.dev/plans).** The hosted
product includes a generous free tier and provides a seamless, reliable,
cost-effective platform that allows you to focus on building your application
without worrying about infrastructure.

That being said, we understand that won't work for everyone. You are welcome to
self-host Convex on your own infrastructure instead.

Community support is available for self-hosted Convex in the `#open-source`
channel in the [Convex Discord](https://discord.gg/convex).

Development of the Convex backend is led by the Convex team. We
[welcome bug fixes](./CONTRIBUTING.md) and
[love receiving feedback](https://discord.gg/convex). We keep this repository
synced with any internal development work within a handful of days.

# Self Hosting Guide

First, you'll want to be get the
[convex-local-backend binary](../crates/local_backend/README.md#running-the-convex-backend).

You can either build from source or use the precompiled binaries. Note that in
order to self host properly, you will need to clone the `convex-backend`
repository and get set up with rust, in order to use some of the administrative
executables.

## Select an instance name

Select a name for your instance. In the cloud product, this takes the form of
something like "flying-fox-123". You can select whatever you want here. A good
practice would be to pick something that matches the domain you host from,
though it is not critical.

## Generate a new instance secret

Instance secret is the secret to the backend. Keep very safe and only accessible
from the backend itself. Generate a new random instance secret with

```sh
cargo run -p keybroker --bin generate_secret
```

It will look like this:
`4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974`

## Generate a new admin key

With the instance name and instance secret, generate an admin key. Admin key is
required to push code to the backend and take other administrator operations.

```sh
cargo run -p keybroker --bin generate_key -- flying-fox-123 4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974
```

It will look like
`flying-fox-123|01c046ab1512d9306a6abda3eedec5dfe862f1fe0f66a5aee774fb9ae3fda87706facaf682b9d4f9209a05e038cbd6e9b8`

## Run your backend instance

Use the instance name and instance secret to start your backend.

```sh
./convex-local-backend --instance-name flying-fox-123 --instance-secret 4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974
```

You can run `./convex-local-backend --help` to see other options for things like
changing ports, convex origin url, convex site url, local storage directories
and other configuration.

## Run your backend instance in Docker

You'll need to have [Docker](https://docs.docker.com/desktop/) installed to run
convex in Docker.

```sh
cd self-hosted
# Pull the latest docker images
docker compose pull
# Run the containers
docker compose up
```

Once your Convex backend is running in Docker, you can ask it to generate admin
keys.

```sh
docker exec convex-local-backend ./generate_admin_key.sh
```

Visit the dashboard at `http://localhost:6791` and use the CLI to push functions
with the `--admin-key $ADMIN_KEY` and `--url http://127.0.0.1:3210` flags.

## Push code to your backend

Using your admin key, push code to your backend. Admin key should be kept secure
to just the developers who are administering the application on your backend.

```sh
cd your_project
npm install
npx convex dev --admin-key 'flying-fox-123|01c046ab1512d9306a6abda3eedec5dfe862f1fe0f66a5aee774fb9ae3fda87706facaf682b9d4f9209a05e038cbd6e9b8' --url "http://127.0.0.1:3210"
```
