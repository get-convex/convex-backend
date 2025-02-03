```sh
# From the self-hosted directory, deploy the backend.
fly launch
# Copy and paste the url that is output to set NEXT_PUBLIC_DEPLOYMENT_URL in the dashboard/fly.toml file.

# Generate admin key
fly ssh console --app self-hosted-backend --command "./generate_admin_key.sh"
cd dashboard
fly launch
# TODO Remove after dashboard docker rebuild
fly secrets set NEXT_PUBLIC_ADMIN_KEY=<your-admin-key>
fly deploy
# Visit the dashboard at the url output from the fly deploy command.

# Write these environment variables to .env.local
CONVEX_SELF_HOST_DEPLOYMENT_URL='<NEXT_PUBLIC_DEPLOYMENT_URL>'
CONVEX_DEPLOY_KEY='<your-admin-key>'
# Push your Convex functions
npx convex deploy
# Visit the dashboard - you should see your functions and be able to edit data, run functions, etc.
```
