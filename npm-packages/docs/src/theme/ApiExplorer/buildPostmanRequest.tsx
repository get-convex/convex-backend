/*
 * Wrap buildPostmanRequest to insert the `Convex` prefix into the Authorization header for Deployment API endpoints.
 * Note that the security schemes for the Deployment API and Management API have the same names, but the Deployment
 * API is configured as using an API key in header, while the Management API uses Bearer Auth.
 */

import buildPostmanRequest from "@theme-original/ApiExplorer/buildPostmanRequest";

// TODO: hardcoded because openapi-typescript doesn't support generating types for securitySchemes yet
const SECURITY_SCHEMES = [
  "Deploy Key",
  "OAuth Project Token",
  "OAuth Team Token",
  "Team Token",
];

export default function buildPostmanRequestWrapper(postman, _options) {
  const options = structuredClone(_options);
  const { auth } = options;
  if (auth.selected === undefined) {
    return buildPostmanRequest(postman, options);
  }

  const selectedAuth = auth.options[auth.selected];
  for (const a of selectedAuth) {
    if (SECURITY_SCHEMES.includes(a.key)) {
      // Deployment API endpoints include an `apiKey` field, while Management API endpoints will include
      // a `token` fields.
      const apiKey = auth.data[a.key].apiKey;
      // We only want to add the `Convex ` prefix for Deployment API endpoints
      if (!apiKey || apiKey.startsWith("Convex")) {
        continue;
      }
      auth.data[a.key].apiKey = `Convex ${apiKey}`;
    }
  }

  return buildPostmanRequest(postman, options);
}
