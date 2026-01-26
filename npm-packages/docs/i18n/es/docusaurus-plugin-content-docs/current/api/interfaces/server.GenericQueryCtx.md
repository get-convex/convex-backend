---
id: "server.GenericQueryCtx"
title: "Interfaz: GenericQueryCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericQueryCtx

Un conjunto de servicios para utilizar dentro de funciones de consulta de Convex.

El contexto de consulta se pasa como el primer argumento a cualquier función de consulta de Convex
que se ejecute en el servidor.

Esto difiere de `MutationCtx` porque todos los servicios son
de solo lectura.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende de [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Propiedades \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseReader`](server.GenericDatabaseReader.md)&lt;`DataModel`&gt;

Una utilidad para leer datos de la base de datos.

#### Definido en \{#defined-in\}

[server/registration.ts:130](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L130)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

Información sobre el usuario actualmente autenticado.

#### Definido en \{#defined-in\}

[server/registration.ts:135](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L135)

***

### storage \{#storage\}

• **storage**: [`StorageReader`](server.StorageReader.md)

Herramienta para leer archivos del almacenamiento.

#### Definido en \{#defined-in\}

[server/registration.ts:140](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L140)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Invoca una función de consulta dentro de la misma transacción.

NOTA: a menudo puedes llamar directamente a la función de la consulta en lugar de usar esto.
`runQuery` introduce la sobrecarga de validar los argumentos y el valor de retorno,
y de crear un nuevo contexto de JS aislado.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Query` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"query"`, `"public"` | `"internal"`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `query` | `Query` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt; |

##### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### Definida en \{#defined-in\}

[server/registration.ts:149](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L149)