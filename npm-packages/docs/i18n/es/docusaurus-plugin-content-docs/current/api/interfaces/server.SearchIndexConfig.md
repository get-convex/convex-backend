---
id: "server.SearchIndexConfig"
title: "Interfaz: SearchIndexConfig<SearchField, FilterFields>"
custom_edit_url: null
---

[server](../modules/server.md).SearchIndexConfig

Configuración de un índice de búsqueda de texto completo.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `SearchField` | extends `string` |
| `FilterFields` | extends `string` |

## Propiedades \{#properties\}

### searchField \{#searchfield\}

• **searchField**: `SearchField`

El campo que se indexará para la búsqueda de texto completo.

Debe ser un campo de tipo `string`.

#### Definido en \{#defined-in\}

[server/schema.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L101)

***

### filterFields \{#filterfields\}

• `Optional` **filterFields**: `FilterFields`[]

Campos adicionales que se indexan para realizar filtrados rápidos al ejecutar consultas de búsqueda.

#### Definido en \{#defined-in\}

[server/schema.ts:106](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L106)