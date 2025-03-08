# Self-hosting Convex with [Railway.com](https://railway.com/)

## Deploying to Railway.com

You can view the ready to deploy [template here](https://railway.com/template/OKpPqB?referralCode=OAdiMp)

or use one click deploy button below:

[![Deploy on Railway.com](https://railway.com/button.svg)](https://railway.com/template/OKpPqB?referralCode=OAdiMp)

## Setup

The template comes with pre-configured env-variables and you need to follow some setup steps to make it fully work.

1. Setting up ports for the deployment
2. Generating admin key with railway ssh

Steps:

1. Setting up ports for the deployment

   Go to your railway deployment

   - Select `convex-backend` service
   - Select Settings tab and scroll to `Public Networking` section
   - Hover on the domain and delete auto assigned domain
   - Click on `Generate Domain` for auto generated one or `Custom Domain` if you want to setup custom domain
   - Make sure to select port `3210` and add your domain
   - Re-deploy both `convex-dashboard` and `convex-backend` services

2. Generating admin key with railway ssh

   Follow [these](https://blog.railway.com/p/ssh#how-to-ssh-on-railway) instructions to setup railway SSH on your machine

   - Link your convex deployment project
   - Run `railway ssh` and select `convex-backend` when prompted
   - Run `ls` and then `./generate_admin_key.sh`
   - Copy the whole admin key logged on the screen
   - This is your admin key keep it secret

### HTTP Actions

Note that HTTP actions run on your railway app url under the `/http` path. For
example:

- If your railway app is deployed at `https://self-hosted-backend.railway.app`
- And you have an HTTP action routed to `/sendEmail`
- You would call it at `https://self-hosted-backend.railway.app/http/sendEmail`

### Database

At this point, your data is stored in SQLite and your files are stored in the
Railway volume. You can see them in the `data` folder if you run:

```
railway ssh
ls
```

To store your data in a SQL database of your choice, see
[these instructions](https://github.com/get-convex/convex-backend/tree/main/self-hosted/README.md#running-the-database-on-postgres--or-mysql).

## Accessing the deployed dashboard

The dashboard allows you to see logs, read/write data, run functions, and more.
You can run the dashboard locally with Docker, or deploy it to Railway.

- Head over to your railway app
- Select convex-dashboard
- Visit its public url
- Paste the admin key when prompted
- Enjoy

### Running the dashboard locally

```sh
docker run -e 'NEXT_PUBLIC_DEPLOYMENT_URL=<backend-url>' -p '6791:6791' 'ghcr.io/get-convex/convex-dashboard:latest'
```

## Deploying your frontend app

See
[these instructions](https://github.com/get-convex/convex-backend/tree/main/self-hosted/README.md#deploying-your-frontend-app).

## Troubleshooting

- **Performance issues**: The default railway configuration allocates the minimum
  possible resources to get up and running. If your app has high load, you may
  see ratelimiting from railway and poor performance. We recommend increasing your
  memory and CPU.
- **Running out of disk space**: The hobby railway configuration allocates 5GB to
  the `convex_data` volume where your SQLite database and storage lives. If you
  run out of space, you can increase the volume to 50GB by upgrading plan.
