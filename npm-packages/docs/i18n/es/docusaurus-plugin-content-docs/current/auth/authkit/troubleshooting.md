---
title: "Solución de problemas de AuthKit"
sidebar_label: "Solución de problemas"
sidebar_position: 30
description: "Depuración de problemas con la autenticación de AuthKit en Convex"
---

## Plataforma no autorizada \{#platform-not-authorized\}

```
WorkOSPlatformNotAuthorized: Your WorkOS platform API key is not authorized to
access this team. Please ensure the API key has the correct permissions in the
WorkOS dashboard.
```

Este error se produce cuando tu clave de API de la plataforma WorkOS no está autorizada para acceder
al equipo de WorkOS asociado con tu equipo de Convex.

Esto suele suceder cuando se ha quitado Convex del espacio de trabajo de WorkOS.

Puedes ponerte en contacto con el equipo de soporte de WorkOS para solicitar que se restablezca este permiso o desvincular el
espacio de trabajo actual y crear uno nuevo:

```bash
npx convex integration workos disconnect-team
npx convex integration workos provision-team
```

Deberás usar una dirección de correo electrónico diferente para crear tu nuevo espacio de trabajo de WorkOS, ya que una dirección de correo electrónico solo puede estar asociada a un único espacio de trabajo de WorkOS.
