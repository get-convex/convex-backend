export default {
  providers: [
    {
      // Go to your Convex dashboard deployment Settings to configure
      // these environment variables.
      domain: process.env.AUTH0_DOMAIN,
      applicationID: process.env.AUTH0_CLIENT_ID,
    },
  ],
};
