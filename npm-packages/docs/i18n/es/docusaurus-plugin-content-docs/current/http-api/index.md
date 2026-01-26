---
title: "API HTTP de Convex"
sidebar_label: "API HTTP pública"
description: "Conexión directa a Convex mediante HTTP"
---

import Tabs from "@theme/Tabs"; import TabItem from "@theme/TabItem";

Las funciones públicas que definen un despliegue se exponen a través de endpoints HTTP públicos.

## Formato de valores de Convex \{#convex-value-format\}

Cada una de las API HTTP acepta un parámetro de consulta `format` que describe cómo se formatean los documentos. Actualmente, el único valor admitido es `json`. Consulta nuestra
[página de tipos](/database/types#convex-values) para más detalles. Ten en cuenta que, por simplicidad, el formato `json` no admite todos los tipos de datos de Convex como entrada y utiliza una representación solapada para varios tipos de datos en la salida. Planeamos añadir un nuevo formato con compatibilidad con todos los tipos de datos de Convex en el futuro.

## Autenticación de la API \{#api-authentication\}

La API de Functions puede autenticarse opcionalmente como un usuario mediante un token *bearer*
en un encabezado `Authorization`. El valor es `Bearer <access_key>`, donde la clave es
un token de tu proveedor de autenticación. Consulta la sección
[under the hood](/auth/clerk#under-the-hood) de la documentación de Clerk para
obtener más detalles sobre cómo funciona esto con Clerk.

Las solicitudes de exportación en streaming e importación en streaming requieren autorización
de administrador del despliegue mediante el encabezado HTTP `Authorization`. El valor es
`Convex <access_key>`, donde la clave de acceso proviene de &quot;Deploy key&quot; en el panel de control
de Convex y otorga acceso completo de lectura y escritura a tus datos de Convex.

## API de funciones \{#functions-api\}

### POST `/api/query`, `/api/mutation`, `/api/action` \{#post-apiquery-apimutation-apiaction\}

Estos endpoints HTTP te permiten llamar a funciones de Convex y obtener el resultado como un
valor.

Puedes encontrar la URL de implementación de tu backend en la página
[Settings](/dashboard/deployments/settings.md) del panel de control, luego la URL de la API será
`<CONVEX_URL>/api/query`, etc., por ejemplo:

<Tabs>
  <TabItem value="shell" label="Shell">
    ```
    curl https://acoustic-panther-728.convex.cloud/api/query \
       -d '{"path": "messages:list", "args": {}, "format": "json"}' \
       -H "Content-Type: application/json"
    ```
  </TabItem>

  <TabItem value="js" label="NodeJS">
    ```js
    const url = "https://acoustic-panther-728.convex.cloud/api/query";
    const request = { path: "messages:list", args: {}, format: "json" };

    const response = fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    ```
  </TabItem>

  <TabItem value="py" label="Python">
    ```py
    import requests

    url = "https://acoustic-panther-728.convex.cloud/api/query"
    headers = {"accept": "application/json"}
    body = {"path": "messages:list", "args": {}, "format": "json"}

    response = requests.post(url, headers=headers, json=body)
    ```
  </TabItem>
</Tabs>

**Parámetros del cuerpo JSON**

| Name   | Type   | Required | Description                                                                                                                         |
| ------ | ------ | -------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| path   | string | y        | Ruta a la función de Convex formateada como cadena de texto, tal como se define [aquí](/functions/query-functions#query-names).   |
| args   | object | y        | Objeto de argumentos con nombre que se pasa a la función de Convex.                                                                |
| format | string | n        | Formato de salida para valores. Valores válidos: [`json`]                                                                          |

**JSON de resultado en caso de éxito**

| Field Name | Type         | Description                                                   |
| ---------- | ------------ | ------------------------------------------------------------- |
| status     | string       | &quot;success&quot;                                                     |
| value      | object       | Resultado de la función de Convex en el formato solicitado.  |
| logLines   | list[string] | Líneas de log generadas durante la ejecución de la función.  |

**JSON de resultado en caso de error**

| Field Name   | Type         | Description                                                                                                                       |
| ------------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                                                           |
| errorMessage | string       | El mensaje de error.                                                                                                              |
| errorData    | object       | Datos de error dentro de un [error de aplicación](/functions/error-handling/application-errors) si se lanzó.                    |
| logLines     | list[string] | Líneas de log generadas durante la ejecución de la función.                                                                      |

### POST `/api/run/{functionIdentifier}` \{#post-apirunfunctionidentifier\}

Este endpoint HTTP te permite llamar a tipos arbitrarios de funciones de Convex utilizando
la ruta en la URL de la solicitud y obtener el resultado como un valor. El identificador
de la función tiene el formato de cadena definido
[aquí](/functions/query-functions#query-names), con una `/` en lugar de `:`.

Puedes encontrar la URL de implementación de tu backend en la página
[Settings](/dashboard/deployments/settings.md) del panel de control; después, la URL de la API será
`<CONVEX_URL>/api/run/{functionIdentifier}`, por ejemplo:

<Tabs>
  <TabItem value="shell" label="Shell">
    ```
    curl https://acoustic-panther-728.convex.cloud/api/run/messages/list \
       -d '{"args": {}, "format": "json"}' \
       -H "Content-Type: application/json"
    ```
  </TabItem>

  <TabItem value="js" label="NodeJS">
    ```js
    const url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list";
    const request = { args: {}, format: "json" };

    const response = fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    ```
  </TabItem>

  <TabItem value="py" label="Python">
    ```py
    import requests

    url = "https://acoustic-panther-728.convex.cloud/api/run/messages/list"
    headers = {"accept": "application/json"}
    body = {"args": {}, "format": "json"}

    response = requests.get(url, headers=headers, body=json)
    ```
  </TabItem>
</Tabs>

**Parámetros del cuerpo JSON**

| Name   | Type   | Required | Description                                                                      |
| ------ | ------ | -------- | -------------------------------------------------------------------------------- |
| args   | object | y        | Objeto de argumentos nombrados que se pasa a la función de Convex.              |
| format | string | n        | Formato de salida para los valores. Predeterminado: `json`. Valores válidos: [`json`] |

**JSON de resultado en caso de éxito**

| Field Name | Type         | Description                                                          |
| ---------- | ------------ | -------------------------------------------------------------------- |
| status     | string       | &quot;success&quot;                                                            |
| value      | object       | Resultado de la función de Convex en el formato solicitado.         |
| logLines   | list[string] | Líneas de registro (log) impresas durante la ejecución de la función. |

**JSON de resultado en caso de error**

| Field Name   | Type         | Description                                                                                                            |
| ------------ | ------------ | ---------------------------------------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                                                |
| errorMessage | string       | El mensaje de error.                                                                                                   |
| errorData    | object       | Datos de error dentro de un [application error](/functions/error-handling/application-errors) si se lanzó.           |
| logLines     | list[string] | Líneas de registro (log) impresas durante la ejecución de la función.                                                 |