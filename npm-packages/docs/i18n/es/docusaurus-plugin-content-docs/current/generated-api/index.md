---
title: Código generado
description:
  "Código JavaScript y TypeScript autogenerado específico para la API de tu aplicación"
---

Convex usa generación de código para crear código específico para el modelo de
datos y la API de tu aplicación. Convex genera archivos JavaScript (`.js`) con
definiciones de tipos de TypeScript (`.d.ts`).

La generación de código no es obligatoria para usar Convex, pero usar el código
generado te dará un autocompletado de código mucho mejor en tu editor y mayor
seguridad de tipos si estás usando TypeScript.

Para generar el código, ejecuta:

```
npx convex dev
```

Esto crea un directorio `convex/_generated` que contiene:

* [`api.js` y `api.d.ts`](./api.md)
* [`dataModel.d.ts`](./data-model.md)
* [`server.js` y `server.d.ts`](./server.md)
