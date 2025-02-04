```sh
# From the self-hosted directory, deploy the backend.
fly launch
# Copy and paste the url that is output to set NEXT_PUBLIC_DEPLOYMENT_URL in the dashboard/fly.toml file.

cd dashboard
fly launch
# Visit the dashboard at the url output from the fly deploy command.
# Generate admin key to login to the dashboard.
fly ssh console --app self-hosted-backend --command "./generate_admin_key.sh"

# Write these environment variables to .env.local
CONVEX_SELF_HOST_DEPLOYMENT_URL='<NEXT_PUBLIC_DEPLOYMENT_URL>'
CONVEX_DEPLOY_KEY='<your-admin-key>'
# Push your Convex functions
npx convex deploy
# Visit the dashboard - you should see your functions and be able to edit data, run functions, etc.
```
