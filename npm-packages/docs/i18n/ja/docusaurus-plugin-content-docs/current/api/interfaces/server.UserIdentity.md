---
id: "server.UserIdentity"
title: "インターフェース: UserIdentity"
custom_edit_url: null
---

[server](../modules/server.md).UserIdentity

認証済みユーザーに関する情報であり、
[JWT](https://datatracker.ietf.org/doc/html/rfc7519) から導出されます。

常に存在することが保証されているフィールドは
[tokenIdentifier](server.UserIdentity.md#tokenidentifier) と [issuer](server.UserIdentity.md#issuer) のみです。残りのフィールドは、
アイデンティティプロバイダが提供する情報に応じて存在したりしなかったりします。

明示的に列挙されているフィールドは OpenID Connect (OIDC) の標準フィールドから導出されています。
これらのフィールドの詳細については
[OIDC specification](https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims)
を参照してください。

それ以外の追加フィールドは JWT に含まれるカスタムクレームであり、
その型はアイデンティティプロバイダの設定に依存します。フィールドの型が分かっている場合は、
次のように TypeScript でその型をアサートできます
（たとえば `string` として）:

```typescript
const identity = await ctx.auth.getUserIdentity();
if (identity === null) {
  return null;
}
const customClaim = identity.custom_claim as string;
```

## インデックスシグネチャ \{#indexable\}

▪ [key: `string`]: [`JSONValue`](../modules/values.md#jsonvalue) | `undefined`

## プロパティ \{#properties\}

### tokenIdentifier \{#tokenidentifier\}

• `Readonly` **tokenIdentifier**: `string`

この ID を表す、安定的でグローバルに一意な文字列です（つまり、別のアイデンティティプロバイダ由来のユーザーであっても、同じ文字列になることはありません）。

JWT クレーム: `sub` + `iss`

#### 定義元 \{#defined-in\}

[server/authentication.ts:107](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L107)

***

### subject \{#subject\}

• `Readonly` **subject**: `string`

認証プロバイダーが発行するエンドユーザーの識別子で、異なるプロバイダー間で一意とは限りません。

JWT クレーム: `sub`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:115](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L115)

***

### issuer \{#issuer\}

• `Readonly` **issuer**: `string`

このユーザーの認証に使用されたアイデンティティプロバイダーのホスト名。

JWT クレーム: `iss`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L122)

***

### name \{#name\}

• `Optional` `Readonly` **name**: `string`

JWT のクレーム: `name`

#### 定義元 \{#defined-in\}

[server/authentication.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L127)

***

### givenName \{#givenname\}

• `Optional` `Readonly` **givenName**: `string`

JWT クレーム: `given_name`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:132](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L132)

***

### familyName \{#familyname\}

• `Optional` `Readonly` **familyName**: `string`

JWT のクレーム: `family_name`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:137](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L137)

***

### nickname \{#nickname\}

• `Optional` `Readonly` **nickname**: `string`

JWT クレーム: `nickname`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:142](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L142)

***

### preferredUsername \{#preferredusername\}

• `Optional` `Readonly` **preferredUsername**: `string`

JWT クレーム: `preferred_username`

#### 定義元 \{#defined-in\}

[server/authentication.ts:147](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L147)

***

### profileUrl \{#profileurl\}

• `Optional` `Readonly` **profileUrl**: `string`

JWT のクレーム: `profile`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:152](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L152)

***

### pictureUrl \{#pictureurl\}

• `Optional` `Readonly` **pictureUrl**: `string`

JWT のクレーム: `picture`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L157)

***

### email \{#email\}

• `Optional` `Readonly` **email**: `string`

JWT クレーム: `email`

#### 定義元 \{#defined-in\}

[server/authentication.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L162)

***

### emailVerified \{#emailverified\}

• `Optional` `Readonly` **emailVerified**: `boolean`

JWT クレーム: `email_verified`

#### 定義元 \{#defined-in\}

[server/authentication.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L167)

***

### gender \{#gender\}

• `Optional` `Readonly` **gender**: `string`

JWT のクレーム: `gender`

#### 定義元 \{#defined-in\}

[server/authentication.ts:172](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L172)

***

### birthday \{#birthday\}

• `Optional` `Readonly` **birthday**: `string`

JWT のクレーム: `birthdate`

#### 定義元 \{#defined-in\}

[server/authentication.ts:177](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L177)

***

### timezone \{#timezone\}

• `Optional` `Readonly` **timezone**: `string`

JWT のクレーム: `zoneinfo`

#### 定義元 \{#defined-in\}

[server/authentication.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L182)

***

### language \{#language\}

• `Optional` `Readonly` **language**: `string`

JWT のクレーム: `locale`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:187](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L187)

***

### phoneNumber \{#phonenumber\}

• `Optional` `Readonly` **phoneNumber**: `string`

JWT のクレーム: `phone_number`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:192](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L192)

***

### phoneNumberVerified \{#phonenumberverified\}

• `Optional` `Readonly` **phoneNumberVerified**: `boolean`

JWT のクレーム: `phone_number_verified`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:197](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L197)

***

### address \{#address\}

• `Optional` `Readonly` **address**: `string`

JWT のクレーム: `address`

#### 定義場所 \{#defined-in\}

[server/authentication.ts:202](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L202)

***

### updatedAt \{#updatedat\}

• `Optional` `Readonly` **updatedAt**: `string`

JWT のクレーム: `updated_at`

#### 定義元 \{#defined-in\}

[server/authentication.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L207)