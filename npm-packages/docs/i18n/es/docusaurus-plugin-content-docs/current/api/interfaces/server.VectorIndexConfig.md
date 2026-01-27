---
id: "server.VectorIndexConfig"
title: "Interfaz: VectorIndexConfig<VectorField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).VectorIndexConfig

La configuración de un índice vectorial.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `VectorField` | extiende `string` |
| `FilterFields` | extiende `string` |

## Propiedades \{#properties\}

### vectorField \{#vectorfield\}

• **vectorField**: `VectorField`

El campo que se indexará para la búsqueda vectorial.

Debe ser un campo de tipo `v.array(v.float64())` (o una unión).

#### Definido en \{#defined-in\}

[server/schema.ts:123](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L123)

***

### dimensiones \{#dimensions\}

• **dimensions**: `number`

La longitud de los vectores indexados. Debe ser un valor entre 2 y 2048, ambos inclusive.

#### Definido en \{#defined-in\}

[server/schema.ts:127](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L127)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

Campos adicionales que se indexan para un filtrado rápido al ejecutar búsquedas vectoriales.

#### Definido en \{#defined-in\}

[server/schema.ts:131](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L131)