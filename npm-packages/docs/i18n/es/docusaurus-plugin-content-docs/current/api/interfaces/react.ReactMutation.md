---
id: "react.ReactMutation"
title: "Interfaz: ReactMutation<Mutation>"
custom_edit_url: null
---

[react](../modules/react.md).ReactMutation

Interfaz para ejecutar una función de mutación de Convex en el servidor.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Mutation` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"mutation"`&gt; |

## Llamable \{#callable\}

### ReactMutation \{#reactmutation\}

▸ **ReactMutation**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

Ejecuta la mutación en el servidor y devuelve una `Promise` de su valor de retorno.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Mutation`&gt; | Argumentos de la mutación que se pasan al servidor. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Mutation`&gt;&gt;

El valor de retorno de la llamada a la función del lado del servidor.

#### Definido en \{#defined-in\}

[react/client.ts:64](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L64)

## Métodos \{#methods\}

### withOptimisticUpdate \{#withoptimisticupdate\}

▸ **withOptimisticUpdate**&lt;`T`&gt;(`optimisticUpdate`): [`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

Define una actualización optimista para aplicar como parte de esta mutación.

Se trata de una actualización temporal de los resultados de las consultas locales para facilitar
una interfaz de usuario rápida e interactiva. Permite que los resultados de las consultas se actualicen antes de que se ejecute
una mutación en el servidor.

Cuando se invoque la mutación, se aplicará la actualización optimista.

Las actualizaciones optimistas también se pueden usar para eliminar temporalmente consultas del
cliente y crear experiencias de carga hasta que una mutación se complete y
los nuevos resultados de las consultas se sincronicen.

La actualización se revertirá automáticamente cuando la mutación se haya completado por completo
y las consultas se hayan actualizado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;[`FunctionArgs`](../modules/server.md#functionargs)&lt;`Mutation`&gt;&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `optimisticUpdate` | `T` &amp; `ReturnType`&lt;`T`&gt; extends `Promise`&lt;`any`&gt; ? `"Optimistic update handlers must be synchronous"` : {} | La actualización optimista que se debe aplicar. |

#### Devuelve \{#returns\}

[`ReactMutation`](react.ReactMutation.md)&lt;`Mutation`&gt;

Una nueva `ReactMutation` con la actualización configurada.

#### Definido en \{#defined-in\}

[react/client.ts:87](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L87)