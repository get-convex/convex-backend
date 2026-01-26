---
id: "server.ValidatedFunction"
title: "Interface: ValidatedFunction&lt;Ctx, ArgsValidator, Returns&gt;"
custom_edit_url: null
---

[server](../modules/server.md).ValidatedFunction

**`Deprecated`**

-- Consulta la definición de tipo de `MutationBuilder` o similares para
los tipos usados al definir funciones de Convex.

Definición de una función de consulta, mutación o acción de Convex con
validación de argumentos.

La validación de argumentos te permite comprobar que los argumentos de esta función
son del tipo esperado.

Ejemplo:

```js
import { query } from "./_generated/server";
import { v } from "convex/values";

export const func = query({
  args: {
    arg: v.string()
  },
  handler: ({ db }, { arg }) => {...},
});
```

**Por motivos de seguridad, se debe añadir validación de argumentos a todas las funciones públicas en
aplicaciones de producción.**

Consulta [UnvalidatedFunction](../modules/server.md#unvalidatedfunction) para funciones sin validación de argumentos.

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `ArgsValidator` | extiende [`PropertyValidators`](../modules/values.md#propertyvalidators) |
| `Returns` | `Returns` |

## Propiedades \{#properties\}

### args \{#args\}

• **args**: `ArgsValidator`

Un validador para los argumentos de esta función.

Es un objeto que mapea nombres de argumentos a validadores construidos con
[v](../modules/values.md#v).

```js
import { v } from "convex/values";

const args = {
  stringArg: v.string(),
  optionalNumberArg: v.optional(v.number()),
}
```

#### Definido en \{#defined-in\}

[server/registration.ts:528](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L528)

***

### handler \{#handler\}

• **handler**: (`ctx`: `Ctx`, `args`: [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt;) =&gt; `Returns`

#### Declaración de tipo \{#type-declaration\}

▸ (`ctx`, `args`): `Returns`

La implementación de esta función.

Esta función recibe el contexto y los argumentos adecuados
y produce un resultado.

##### Parámetros \{#parameters\}

| Name | Type | Description |
| :------ | :------ | :------ |
| `ctx` | `Ctx` | El objeto de contexto. Será uno de QueryCtx, MutationCtx o ActionCtx, según el tipo de función. |
| `args` | [`ObjectType`](../modules/values.md#objecttype)&lt;`ArgsValidator`&gt; | El objeto de argumentos para esta función. Coincide con el tipo definido por el validador de argumentos. |

##### Valor de retorno \{#returns\}

`Returns`

#### Definido en \{#defined-in\}

[server/registration.ts:542](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L542)