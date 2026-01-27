---
id: "server.Auth"
title: "Interfaz: Auth"
custom_edit_url: null
---

[server](../modules/server.md).Auth

Una interfaz para acceder a la información sobre el usuario actualmente autenticado
dentro de las funciones de consulta y mutación de Convex.

## Métodos \{#methods\}

### getUserIdentity \{#getuseridentity\}

▸ **getUserIdentity**(): `Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

Obtiene detalles sobre el usuario autenticado actualmente.

#### Devuelve \{#returns\}

`Promise`&lt;`null` | [`UserIdentity`](server.UserIdentity.md)&gt;

Una promesa que se resuelve con un [UserIdentity](server.UserIdentity.md) si el cliente de Convex
se configuró con un token de ID válido o, si no, hará lo siguiente:

* devuelve `null` en consultas, mutaciones y acciones de Convex.
* lanza (`throw`) en acciones HTTP.

#### Definido en \{#defined-in\}

[server/authentication.ts:236](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L236)