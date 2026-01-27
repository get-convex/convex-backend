---
title: "api.js"
sidebar_position: 2
description:
  "Referencias de API generadas para tus funciones de Convex y llamadas internas"
---

<Admonition type="caution" title="Este código se genera automáticamente">
  Estas exportaciones no están disponibles directamente en el paquete `convex`.

  En su lugar, debes ejecutar `npx convex dev` para crear `convex/_generated/api.js`
  y `convex/_generated/api.d.ts`.
</Admonition>

Estos tipos requieren ejecutar la generación de código porque son específicos de las
funciones de Convex que defines para tu aplicación.

Si no utilizas la generación de código, puedes usar
[`makeFunctionReference`](/api/modules/server#makefunctionreference) en su lugar.

### api \{#api\}

Un objeto de tipo `API` que describe la API pública de Convex de tu aplicación.

Su tipo `API` incluye información sobre los argumentos y los tipos de retorno de las
funciones de Convex de tu aplicación.

El objeto api es utilizado por hooks de React en el lado del cliente y por funciones de Convex que ejecutan
o programan otras funciones.

```javascript title="src/App.jsx"
import { api } from "../convex/_generated/api";
import { useQuery } from "convex/react";

const data = useQuery(api.messages.list);
```

### internal \{#internal\}

Otro objeto de tipo `API` que describe la API interna de Convex de tu aplicación.

```js title="convex/upgrade.js"
import { action } from "../_generated/server";
import { internal } from "../_generated/api";

export default action({
  handler: async ({ runMutation }, { planId, ... }) => {
    // Llamar al proveedor de pagos (p. ej., Stripe) para cobrar al cliente
    const response = await fetch(...);
    if (response.ok) {
      // Marcar el plan como "profesional" en la BD de Convex
      await runMutation(internal.plans.markPlanAsProfessional, { planId });
    }
  },
});
```
