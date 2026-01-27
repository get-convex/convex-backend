---
id: "server.UserIdentity"
title: "Interfaz: UserIdentity"
custom_edit_url: null
---

[server](../modules/server.md).UserIdentity

Información sobre un usuario autenticado, obtenida a partir de un
[JWT](https://datatracker.ietf.org/doc/html/rfc7519).

Los únicos campos cuya presencia está garantizada son
[tokenIdentifier](server.UserIdentity.md#tokenidentifier) e [issuer](server.UserIdentity.md#issuer). Todos los
campos restantes pueden o no estar presentes dependiendo de la información proporcionada
por el proveedor de identidad.

Los campos listados explícitamente se derivan de los campos estándar de OpenID Connect (OIDC);
consulta la [especificación de OIDC](https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims)
para obtener más información sobre estos campos.

Cualquier campo adicional es una claim personalizada que puede estar presente en el JWT,
y su tipo depende de la configuración de tu proveedor de identidad. Si conoces el tipo
del campo, puedes hacer una aserción de tipo en TypeScript de la siguiente manera
(por ejemplo, como un `string`):

```typescript
const identity = await ctx.auth.getUserIdentity();
if (identity === null) {
  return null;
}
const customClaim = identity.custom_claim as string;
```

## Indexable \{#indexable\}

▪ [key: `string`]: [`JSONValue`](../modules/values.md#jsonvalue) | `undefined`

## Propiedades \{#properties\}

### tokenIdentifier \{#tokenidentifier\}

• `Readonly` **tokenIdentifier**: `string`

Una cadena estable y globalmente única para esta identidad (es decir, ningún otro
usuario, ni siquiera de un proveedor de identidad diferente, tendrá la misma cadena).

JWT claims: `sub` + `iss`

#### Definido en \{#defined-in\}

[server/authentication.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L107)

***

### subject \{#subject\}

• `Readonly` **subject**: `string`

Identificador del usuario final del proveedor de identidad, no necesariamente único entre distintos proveedores.

JWT claim: `sub`

#### Definido en \{#defined-in\}

[server/authentication.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L115)

***

### issuer \{#issuer\}

• `Readonly` **issuer**: `string`

El nombre de host del proveedor de identidad que se utilizó para autenticar a este usuario.

JWT claim: `iss`

#### Definido en \{#defined-in\}

[server/authentication.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L122)

***

### name \{#name\}

• `Optional` `Readonly` **name**: `string`

Claim de JWT: `name`

#### Definido en \{#defined-in\}

[server/authentication.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L127)

***

### givenName \{#givenname\}

• `Opcional` `Solo lectura` **givenName**: `string`

Claim de JWT: `given_name`

#### Definido en \{#defined-in\}

[server/authentication.ts:132](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L132)

***

### familyName \{#familyname\}

• `Opcional` `De solo lectura` **familyName**: `string`

JWT claim: `family_name`

#### Definido en \{#defined-in\}

[server/authentication.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L137)

***

### nickname \{#nickname\}

• `Optional` `Readonly` **nickname**: `string`

Claim de JWT: `nickname`

#### Definido en \{#defined-in\}

[server/authentication.ts:142](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L142)

***

### preferredUsername \{#preferredusername\}

• `Optional` `Readonly` **preferredUsername**: `string`

Claim de JWT: `preferred_username`

#### Definido en \{#defined-in\}

[server/authentication.ts:147](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L147)

***

### profileUrl \{#profileurl\}

• `Opcional` `Solo lectura` **profileUrl**: `string`

Claim JWT: `profile`

#### Definido en \{#defined-in\}

[server/authentication.ts:152](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L152)

***

### pictureUrl \{#pictureurl\}

• `Optional` `Readonly` **pictureUrl**: `string`

Claim de JWT: `picture`

#### Definido en \{#defined-in\}

[server/authentication.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L157)

***

### email \{#email\}

• `Optional` `Readonly` **email**: `string`

Claim JWT: `email`

#### Definido en \{#defined-in\}

[server/authentication.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L162)

***

### emailVerified \{#emailverified\}

• `Optional` `Readonly` **emailVerified**: `boolean`

Claim JWT: `email_verified`

#### Definido en \{#defined-in\}

[server/authentication.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L167)

***

### gender \{#gender\}

• `Optional` `Readonly` **gender**: `string`

Claim de JWT: `gender`

#### Definido en \{#defined-in\}

[server/authentication.ts:172](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L172)

***

### birthday \{#birthday\}

• `Optional` `Readonly` **birthday**: `string`

Claim de JWT: `birthdate`

#### Definido en \{#defined-in\}

[server/authentication.ts:177](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L177)

***

### timezone \{#timezone\}

• `Optional` `Readonly` **timezone**: `string`

Claim de JWT: `zoneinfo`

#### Definido en \{#defined-in\}

[server/authentication.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L182)

***

### language \{#language\}

• `Optional` `Readonly` **language**: `string`

Claim de JWT: `locale`

#### Definido en \{#defined-in\}

[server/authentication.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L187)

***

### phoneNumber \{#phonenumber\}

• `Optional` `Readonly` **phoneNumber**: `string`

Claim JWT: `phone_number`

#### Definido en \{#defined-in\}

[server/authentication.ts:192](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L192)

***

### phoneNumberVerified \{#phonenumberverified\}

• `Optional` `Readonly` **phoneNumberVerified**: `boolean`

Declaración JWT: `phone_number_verified`

#### Definido en \{#defined-in\}

[server/authentication.ts:197](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L197)

***

### address \{#address\}

• `Optional` `Readonly` **address**: `string`

Claim de JWT: `address`

#### Definido en \{#defined-in\}

[server/authentication.ts:202](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L202)

***

### updatedAt \{#updatedat\}

• `Optional` `Readonly` **updatedAt**: `string`

Reclamación (claim) de JWT: `updated_at`

#### Definido en \{#defined-in\}

[server/authentication.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L207)