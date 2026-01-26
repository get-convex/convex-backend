---
id: "browser.MutationOptions"
title: "Interfaz: MutationOptions"
custom_edit_url: null
---

[browser](../modules/browser.md).MutationOptions

Opciones para la [mutation](../classes/browser.BaseConvexClient.md#mutation).

## Propiedades \{#properties\}

### optimisticUpdate \{#optimisticupdate\}

• `Optional` **optimisticUpdate**: [`OptimisticUpdate`](../modules/browser.md#optimisticupdate)&lt;`any`&gt;

Una actualización optimista para aplicar junto con esta mutación.

Una actualización optimista actualiza localmente las consultas mientras una mutación está pendiente.
Una vez que la mutación se complete, la actualización se revertirá.

#### Definido en \{#defined-in\}

[browser/sync/client.ts:210](https://github.com/get-convex/convex-js/blob/main/src/browser/sync/client.ts#L210)