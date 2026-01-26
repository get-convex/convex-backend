---
id: "values.Base64"
title: "命名空间：Base64"
custom_edit_url: null
---

[values](../modules/values.md).Base64

## 函数 \{#functions\}

### byteLength \{#bytelength\}

▸ **byteLength**(`b64`): `number`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `b64` | `string` |

#### 返回 \{#returns\}

`number`

#### 定义于 \{#defined-in\}

[values/base64.ts:44](https://github.com/get-convex/convex-js/blob/main/src/values/base64.ts#L44)

***

### toByteArray \{#tobytearray\}

▸ **toByteArray**(`b64`): `Uint8Array`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `b64` | `string` |

#### 返回值 \{#returns\}

`Uint8Array`

#### 定义于 \{#defined-in\}

[values/base64.ts:56](https://github.com/get-convex/convex-js/blob/main/src/values/base64.ts#L56)

***

### fromByteArray \{#frombytearray\}

▸ **fromByteArray**(`uint8`): `string`

#### 参数 \{#parameters\}

| 名称 | 类型 |
| :------ | :------ |
| `uint8` | `Uint8Array` |

#### 返回值 \{#returns\}

`string`

#### 定义于 \{#defined-in\}

[values/base64.ts:123](https://github.com/get-convex/convex-js/blob/main/src/values/base64.ts#L123)

***

### fromByteArrayUrlSafeNoPadding \{#frombytearrayurlsafenopadding\}

▸ **fromByteArrayUrlSafeNoPadding**(`uint8`): `string`

#### 参数 \{#parameters\}

| 参数名 | 类型 |
| :------ | :------ |
| `uint8` | `Uint8Array` |

#### 返回值 \{#returns\}

`string`

#### 定义于 \{#defined-in\}

[values/base64.ts:158](https://github.com/get-convex/convex-js/blob/main/src/values/base64.ts#L158)