---
id: "server.GenericMutationCtx"
title: "Interfaz: GenericMutationCtx<DataModel>"
custom_edit_url: null
---

[server](../modules/server.md).GenericMutationCtx

Un conjunto de servicios para usar dentro de las funciones de mutación de Convex.

El contexto de la mutación se pasa como primer argumento a cualquier función de
mutación de Convex que se ejecute en el servidor.

Si usas generación de código, utiliza el tipo `MutationCtx` en
`convex/_generated/server.d.ts`, que está tipado para tu modelo de datos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](../modules/server.md#genericdatamodel) |

## Propiedades \{#properties\}

### db \{#db\}

• **db**: [`GenericDatabaseWriter`](server.GenericDatabaseWriter.md)&lt;`DataModel`&gt;

Herramienta para leer y escribir datos en la base de datos.

#### Definido en \{#defined-in\}

[server/registration.ts:50](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L50)

***

### auth \{#auth\}

• **auth**: [`Auth`](server.Auth.md)

Información sobre el usuario actualmente autenticado.

#### Definido en \{#defined-in\}

[server/registration.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L55)

***

### storage \{#storage\}

• **storage**: [`StorageWriter`](server.StorageWriter.md)

Una utilidad para leer y escribir archivos en el almacenamiento.

#### Definido en \{#defined-in\}

[server/registration.ts:60](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L60)

***

### scheduler \{#scheduler\}

• **scheduler**: [`Scheduler`](server.Scheduler.md)

Utilidad para programar la ejecución futura de funciones de Convex.

#### Definido en \{#defined-in\}

[server/registration.ts:65](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L65)

***

### runQuery \{#runquery\}

• **runQuery**: &lt;Query&gt;(`query`: `Query`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Query`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`Query`&gt;(`query`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Query`&gt;&gt;

Llama a una función de consulta dentro de la misma transacción.

NOTA: a menudo puedes llamar a la función de la consulta directamente en lugar de usar esto.
`runQuery` añade la sobrecarga de ejecutar la validación de los argumentos y del valor de retorno,
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

#### Definido en \{#defined-in\}

[server/registration.ts:74](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L74)

***

### runMutation \{#runmutation\}

• **runMutation**: &lt;Mutation&gt;(`mutation`: `Mutation`, ...`args`: [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt;) =&gt; `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`Mutation`&gt;(`mutation`, `...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Llama a una función de mutación dentro de la misma transacción.

NOTA: en muchos casos puedes llamar directamente a la función de la mutación en lugar de usar esto.
`runMutation` incurre en la sobrecarga de realizar la validación de los argumentos y del valor de retorno,
y de crear un nuevo contexto de JS aislado.

La mutación se ejecuta en una subtransacción, por lo que, si la mutación lanza un error,
todas sus escrituras se revertirán. Además, las escrituras de una mutación exitosa
serán serializables con otras escrituras en la transacción.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`, `"public"` | `"internal"`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `mutation` | `Mutation` |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; |

##### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:90](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L90)