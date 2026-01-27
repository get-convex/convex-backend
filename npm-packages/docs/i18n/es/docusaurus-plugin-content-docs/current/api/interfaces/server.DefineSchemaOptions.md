---
id: "server.DefineSchemaOptions"
title: "Interfaz: DefineSchemaOptions<StrictTableNameTypes>"
custom_edit_url: null
---

[server](../modules/server.md).DefineSchemaOptions

Opciones de [defineSchema](../modules/server.md#defineschema).

## Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `StrictTableNameTypes` | extends `boolean` |

## Propiedades \{#properties\}

### schemaValidation \{#schemavalidation\}

• `Opcional` **schemaValidation**: `boolean`

Indica si Convex debe validar en tiempo de ejecución que todos los documentos
cumplan tu esquema.

Si `schemaValidation` es `true`, Convex:

1. Comprueba que todos los documentos existentes cumplan tu esquema cuando haces push de tu esquema.
2. Comprueba que todas las inserciones y actualizaciones cumplan tu esquema durante las mutaciones.

Si `schemaValidation` es `false`, Convex no validará que los documentos nuevos o
existentes cumplan tu esquema. Seguirás obteniendo tipos de TypeScript
específicos de tu esquema, pero no habrá validación en tiempo de ejecución de que tus
documentos cumplan esos tipos.

De forma predeterminada, `schemaValidation` es `true`.

#### Definido en \{#defined-in\}

[server/schema.ts:727](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L727)

***

### strictTableNameTypes \{#stricttablenametypes\}

• `Optional` **strictTableNameTypes**: `StrictTableNameTypes`

Indica si los tipos de TypeScript deben permitir acceder a tablas que no están en el esquema.

Si `strictTableNameTypes` es `true`, usar tablas que no estén listadas en el esquema
generará un error de compilación de TypeScript.

Si `strictTableNameTypes` es `false`, podrás acceder a tablas que no estén
listadas en el esquema y su tipo de documento será `any`.

`strictTableNameTypes: false` es útil para la creación rápida de prototipos.

Independientemente del valor de `strictTableNameTypes`, tu esquema solo
validará documentos en las tablas listadas en el esquema. Aun así, puedes crear
y modificar otras tablas en el panel de control o en mutaciones de JavaScript.

De forma predeterminada, `strictTableNameTypes` es `true`.

#### Definido en \{#defined-in\}

[server/schema.ts:746](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L746)