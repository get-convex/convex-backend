---
title: "Custom JWT Provider"
sidebar_label: "Custom JWT Provider"
sidebar_position: 4
---

**Note: This is an advanced feature!** We recommend sticking with the
[supported third-party authentication providers](/auth.mdx).

If your custom auth provider implements the OIDC protocol it's easiest to
configure an [OIDC Provider](/auth/advanced/custom-auth) entry in
`convex/auth.config.ts`. However some third party auth providers only issue JWTs
and don't participate in the full OIDC protocol. For example,
[OpenAuth](https://openauth.js.org/) implements the OAuth 2.0 spec but not OIDC,
so to use it with Convex you'll need to set it up as a Custom JWT provider.

A [JWT](https://en.wikipedia.org/wiki/JSON_Web_Token) is a string combining
three base64-encoded JSON objects containing claims about who a user is, valid
for a limited period of time like one hour. You can build them with a library
like [jose](https://github.com/panva/jose) or get them from a third party
authentication service like [Clerk](https://clerk.com). The information in a JWT
is signed (the Convex deployment can tell the information is really from the
issuer and hasn't been modified) but generally not encrypted (you can see what
one says by base64 decoding the token or pasting it into
[jwt.io](https://jwt.io/).

The JWT header must include at least the `kid`, `alg`, and `typ` fields.

The JWT payload must contain at least the `sub`, `iss`, `iat`, and `exp` fields.

## Server-side integration

Use `type: "customJwt"` to configure a Custom JWT auth provider:

```js noDialect title="convex/auth.config.js"
export default {
  providers: [
    {
      type: "customJwt",
      applicationID: "your-application-id",
      issuer: "https://your.issuer.url.com",
      jwks: "https://your.issuer.url.com/.well-known/jwks.json",
      algorithm: "RS256",
    },
  ],
};
```

- `applicationID` (optional): If provided, Convex will verify that JWTs have
  this value in the `aud` claim.
- `issuer`: The issuer URL of the JWT.
- `jwks`: The URL for fetching the JWKS (JSON Web Key Set) from the auth
  provider. To avoid hitting an external service you may use a data URI, e.g.
  `"data:text/plain;charset=utf-8;base64,ey..."`
- `algorithm`: The algorithm used to sign the JWT. Only RS256 and ES256 are
  currently supported. See
  [RFC 7518](https://datatracker.ietf.org/doc/html/rfc7518#section-3.1) for more
  details.

The `issuer` property must exactly match the `iss` field of the JWT used and if
specified the `applicationID` property must exactly match the `aud` field. If
your JWT doesn't match, use a tool like [jwt.io](https://jwt.io/) to view an JWT
and confirm these fields match exactly.

When adding a custom JWT provider it is your responsibility to ensure the fields
uniquely identify a user to avoid allowing a similar user of a different
application to log in. If the `iss` field / the `issuer` property does not
uniquely identify your app it's important to use the `applicationID` field in
the provider which makes matching `aud` fields required.

### Custom claims

In addition to top-level fields like `subject`, `issuer`, and `tokenIdentifier`,
subfields of the nested fields of the JWT will be accessible in the auth data
returned from `const authInfo = await ctx.auth.getUserIdentity()` like
`authInfo["properties.id"]` and `authInfo["properties.favoriteColor"]` for a JWT
structured like this:

```json
{
  "properties": {
    "id": "123",
    "favoriteColor": "red"
  },
  "iss": "http://localhost:3000",
  "sub": "user:8fa2be73c2229e85",
  "exp": 1750968478
}
```

## Client-side integration

Your user's browser needs a way to obtain an initial JWT and to request updated
JWTs, ideally before the previous one expires.

See the instructions for
[Custom OIDC Providers](/auth/advanced/custom-auth#client-side-integration) for
how to do this.
