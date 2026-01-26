---
id: "server.UserIdentity"
title: "接口：UserIdentity"
custom_edit_url: null
---

[server](../modules/server.md).UserIdentity

关于已通过身份验证的用户的信息，来源于
[JWT](https://datatracker.ietf.org/doc/html/rfc7519)。

唯一保证一定存在的字段是
[tokenIdentifier](server.UserIdentity.md#tokenidentifier) 和 [issuer](server.UserIdentity.md#issuer)。其余所有字段是否存在
取决于身份提供方提供的信息。

下面显式列出的字段来自 OpenID Connect (OIDC) 的标准字段，
有关这些字段的更多信息，请参阅 [OIDC 规范](https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims)。

任何其他附加字段都是可能出现在 JWT 中的自定义声明，
其类型取决于你的身份提供方配置。如果你知道某个字段的类型，
可以在 TypeScript 中像下面这样进行断言
（例如将其断言为 `string`）：

```typescript
const identity = await ctx.auth.getUserIdentity();
if (identity === null) {
  return null;
}
const customClaim = identity.custom_claim as string;
```

## 可索引 \{#indexable\}

▪ [key: `string`]: [`JSONValue`](../modules/values.md#jsonvalue) | `undefined`

## 属性 \{#properties\}

### tokenIdentifier \{#tokenidentifier\}

• `Readonly` **tokenIdentifier**: `string`

此身份对应的稳定且全局唯一的字符串（即使是来自不同身份提供商的其他用户，也不会拥有相同的字符串。）

JWT 声明字段：`sub` + `iss`

#### 定义于 \{#defined-in\}

[server/authentication.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L107)

***

### subject \{#subject\}

• `Readonly` **subject**: `string`

来自身份提供者的终端用户标识符，在不同提供者之间不一定唯一。

JWT claim: `sub`

#### 定义于 \{#defined-in\}

[server/authentication.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L115)

***

### issuer \{#issuer\}

• `Readonly` **issuer**: `string`

用于对该用户进行身份验证的身份提供方的主机名。

JWT claim: `iss`

#### 定义在 \{#defined-in\}

[server/authentication.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L122)

***

### name \{#name\}

• `Optional` `Readonly` **name**: `string`

JWT 声明：`name`

#### 定义于 \{#defined-in\}

[server/authentication.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L127)

***

### givenName \{#givenname\}

• `Optional` `Readonly` **givenName**: `string`

JWT 声明：`given_name`

#### 定义于 \{#defined-in\}

[server/authentication.ts:132](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L132)

***

### familyName \{#familyname\}

• `Optional` `Readonly` **familyName**: `string`

JWT 声明: `family_name`

#### 定义于 \{#defined-in\}

[server/authentication.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L137)

***

### nickname \{#nickname\}

• `可选` `只读` **nickname**: `string`

JWT 声明：`nickname`

#### 定义于 \{#defined-in\}

[server/authentication.ts:142](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L142)

***

### preferredUsername \{#preferredusername\}

• `Optional` `Readonly` **preferredUsername**: `string`

JWT 声明：`preferred_username`

#### 定义于 \{#defined-in\}

[server/authentication.ts:147](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L147)

***

### profileUrl \{#profileurl\}

• `Optional` `Readonly` **profileUrl**: `string`

JWT 声明：`profile`

#### 定义于 \{#defined-in\}

[server/authentication.ts:152](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L152)

***

### pictureUrl \{#pictureurl\}

• `Optional` `Readonly` **pictureUrl**: `string`

JWT 声明：`picture`

#### 定义于 \{#defined-in\}

[server/authentication.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L157)

***

### email \{#email\}

• `可选` `只读` **email**: `string`

JWT 声明：`email`

#### 定义于 \{#defined-in\}

[server/authentication.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L162)

***

### emailVerified \{#emailverified\}

• `Optional` `Readonly` **emailVerified**: `boolean`

JWT 声明：`email_verified`

#### 定义于 \{#defined-in\}

[server/authentication.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L167)

***

### gender \{#gender\}

• `可选` `只读` **gender**: `string`

JWT 声明字段：`gender`

#### 定义于 \{#defined-in\}

[server/authentication.ts:172](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L172)

***

### 生日 \{#birthday\}

• `Optional` `Readonly` **birthday**: `string`

JWT 声明：`birthdate`

#### 定义于 \{#defined-in\}

[server/authentication.ts:177](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L177)

***

### timezone \{#timezone\}

• `可选` `只读` **timezone**: `string`

JWT Claim：`zoneinfo`

#### 定义于 \{#defined-in\}

[server/authentication.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L182)

***

### language \{#language\}

• `可选` `只读` **language**: `string`

JWT 声明：`locale`

#### 定义于 \{#defined-in\}

[server/authentication.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L187)

***

### phoneNumber \{#phonenumber\}

• `Optional` `Readonly` **phoneNumber**: `string`

JWT 声明：`phone_number`

#### 定义于 \{#defined-in\}

[server/authentication.ts:192](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L192)

***

### phoneNumberVerified \{#phonenumberverified\}

• `Optional` `Readonly` **phoneNumberVerified**: `boolean`

JWT 声明字段：`phone_number_verified`

#### 定义于 \{#defined-in\}

[server/authentication.ts:197](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L197)

***

### address \{#address\}

• `可选` `只读` **address**: `string`

JWT 声明：`address`

#### 定义于 \{#defined-in\}

[server/authentication.ts:202](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L202)

***

### updatedAt \{#updatedat\}

• `Optional` `Readonly` **updatedAt**: `string`

JWT 声明：`updated_at`

#### 定义于 \{#defined-in\}

[server/authentication.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L207)