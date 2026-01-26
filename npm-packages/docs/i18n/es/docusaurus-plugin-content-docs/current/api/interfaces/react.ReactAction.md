---
id: "react.ReactAction"
title: "Interfaz: ReactAction<Action>"
custom_edit_url: null
---

[react](../modules/react.md).ReactAction

Una interfaz para ejecutar una acción de Convex en el servidor.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Action` | extends [`FunctionReference`](../modules/server.md#functionreference)&lt;`"action"`&gt; |

## Invocable \{#callable\}

### ReactAction \{#reactaction\}

▸ **ReactAction**(`...args`): `Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

Ejecuta la función en el servidor y devuelve una `Promise` con su valor de retorno.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `...args` | [`OptionalRestArgs`](../modules/server.md#optionalrestargs)&lt;`Action`&gt; | Argumentos de la función que se pasarán al servidor. |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionReturnType`](../modules/server.md#functionreturntype)&lt;`Action`&gt;&gt;

El valor devuelto por la llamada a la función del lado del servidor.

#### Definido en \{#defined-in\}

[react/client.ts:136](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L136)