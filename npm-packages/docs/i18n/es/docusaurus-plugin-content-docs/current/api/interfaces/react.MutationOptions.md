---
id: "react.MutationOptions"
title: "Interfaz: MutationOptions<Args>"
custom_edit_url: null
---

[react](../modules/react.md).MutationOptions

Opciones de [mutación](../classes/react.ConvexReactClient.md#mutation).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Args` | extends `Record`&lt;`string`, [`Value`](../modules/values.md#value)&gt; |

## Propiedades \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`Args`&gt;

Una actualización optimista que se aplicará junto con esta mutación.

Una actualización optimista actualiza localmente las consultas mientras una mutación está pendiente.
Una vez que la mutación se completa, la actualización se revertirá.

#### Definido en \{#defined-in\}

[react/client.ts:282](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L282)