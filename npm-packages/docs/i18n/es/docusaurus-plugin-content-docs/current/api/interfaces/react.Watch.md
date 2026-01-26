---
id: "react.Watch"
title: "Interfaz: Watch<T>"
custom_edit_url: null
---

[react](../modules/react.md).Watch

Un observador de la salida de una función de consulta de Convex.

## Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `T` |

## Métodos \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**(`callback`): () =&gt; `void`

Inicia el seguimiento de la salida de una consulta.

Esto se suscribirá a esta consulta y llamará
al callback cada vez que cambie el resultado de la consulta.

**Importante: Si el cliente ya está suscrito a esta consulta con los
mismos argumentos, este callback no se invocará hasta que el resultado de la consulta se
actualice.** Para obtener el resultado local actual, llama a
[localQueryResult](react.Watch.md#localqueryresult).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `callback` | () =&gt; `void` | Función que se ejecuta cada vez que cambia el resultado de la consulta. |

#### Returns \{#returns\}

`fn`

* Una función que cancela la suscripción.

▸ (): `void`

Inicia un seguimiento del resultado de una consulta.

Esto suscribe esta consulta y llama
al callback cada vez que cambie el resultado de la consulta.

**Importante: si el cliente ya está suscrito a esta consulta con los
mismos argumentos, este callback no se invocará hasta que el resultado de la consulta
se actualice.** Para obtener el resultado local actual, llama a
[localQueryResult](react.Watch.md#localqueryresult).

##### Devuelve \{#returns\}

`void`

* Una función que cancela la suscripción.

#### Definido en \{#defined-in\}

[react/client.ts:170](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L170)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(): `undefined` | `T`

Obtiene el resultado actual de una consulta.

Esto solo devolverá un resultado si ya estás suscrito a la consulta
y has recibido un resultado del servidor, o si el valor de la consulta se ha establecido
de forma optimista.

**`Throws`**

Un error si la consulta encontró un error en el servidor.

#### Devuelve \{#returns\}

`undefined` | `T`

El resultado de la consulta o `undefined` si se desconoce.

#### Definido en \{#defined-in\}

[react/client.ts:182](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L182)

***

### journal \{#journal\}

▸ **journal**(): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

Obtiene el [QueryJournal](../modules/browser.md#queryjournal) actual para esta consulta.

Si aún no se ha recibido ningún resultado para esta consulta, será `undefined`.

#### Devuelve \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

#### Definido en \{#defined-in\}

[react/client.ts:194](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L194)