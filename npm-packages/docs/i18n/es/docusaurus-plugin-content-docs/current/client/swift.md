---
title: "Swift para iOS y macOS"
sidebar_label: "Swift"
sidebar_position: 700
description: "Biblioteca cliente en Swift para aplicaciones iOS y macOS que usan Convex"
---

La biblioteca cliente Swift de Convex permite que tu aplicación iOS o macOS
interactúe con tu backend de Convex. Permite que tu código de frontend:

1. Llame
   a tus [consultas](/functions/query-functions.mdx), [mutaciones](/functions/mutation-functions.mdx) y [acciones](/functions/actions.mdx)
2. Autentique usuarios con [Auth0](/auth/auth0.mdx)

La biblioteca es de código abierto
y [está disponible en GitHub](https://github.com/get-convex/convex-swift).

Sigue el [Quickstart de Swift](/quickstart/swift.mdx) para comenzar.

## Instalación \{#installation\}

Para un proyecto de iOS o macOS en Xcode, debes realizar los siguientes pasos
para añadir una dependencia de la biblioteca `ConvexMobile`.

1. Haz clic en el contenedor de la app de nivel superior en el navegador de proyectos a la izquierda

2. Haz clic en el nombre de la app bajo el encabezado PROJECT

3. Haz clic en la pestaña *Package Dependencies*

4. Haz clic en el botón +

   ![Captura de pantalla 2024-10-02 a las 2:33:43 p. m..png](/screenshots/swift_qs_step_2.png)

5. Pega
   [`https://github.com/get-convex/convex-swift`](https://github.com/get-convex/convex-swift)
   en el cuadro de búsqueda y pulsa Intro

6. Cuando se cargue el paquete `convex-swift`, haz clic en el botón Add Package

7. En el cuadro de diálogo *Package Products*, selecciona el nombre de tu producto en el menú desplegable *Add to Target*

8. Haz clic en *Add Package*

## Conectarse con un backend \{#connecting-to-a-backend\}

`ConvexClient` se utiliza para establecer y mantener una conexión entre tu
aplicación y el backend de Convex. Primero necesitas crear una instancia del
cliente proporcionándole la URL de implementación de tu backend:

```swift
import ConvexMobile

let convex = ConvexClient(deploymentUrl: "https://<your domain here>.convex.cloud")
```

Debes crear y usar una única instancia de `ConvexClient` durante todo el ciclo de vida
de tu aplicación. Puedes almacenar el cliente en una constante global tal como
se muestra arriba. No se iniciará una conexión real con el backend de Convex hasta que
llames a un método de `ConvexClient`. A partir de entonces, mantendrá la
conexión y la restablecerá si se interrumpe.

## Obtención de datos \{#fetching-data\}

La biblioteca de Convex para Swift te da acceso al motor de sincronización de Convex, que
permite *suscripciones* en tiempo real a los resultados de consultas. Te suscribes a consultas
con el método `subscribe` de `ConvexClient`, que devuelve
un [`Publisher`](https://developer.apple.com/documentation/combine). Los datos
disponibles a través del `Publisher` irán cambiando con el tiempo a medida que cambien los datos subyacentes
que respaldan la consulta.

Puedes llamar a métodos en el `Publisher` para transformar y consumir los datos que
proporciona.

Una forma sencilla de consumir una consulta que devuelve una lista de cadenas en una `View`
es usar una combinación de un `@State` que contenga una lista y el modificador `.task`
con código que itere sobre los resultados de la consulta como un `AsyncSequence`:

```swift
struct ColorList: View {
  @State private var colors: [String] = []

  var body: some View {
    List {
      ForEach(colors, id: \.self) { color in
        Text(color)
      }
    }.task {
      let latestColors = convex.subscribe(to: "colors:get", yielding: [String].self)
        .replaceError(with: [])
        .values
      for await colors in latestColors {
        self.colors = colors
      }
    }
  }
}
```

Cada vez que cambien los datos que alimentan la consulta de backend `"colors:get"`, se generará un nuevo array de valores `String` en el `AsyncSequence` y la lista `colors` de la `View` se asignará a estos nuevos datos. La interfaz de usuario se reconstruirá de forma reactiva para reflejar los cambios.

### Argumentos de consulta \{#query-arguments\}

Puedes pasar argumentos a `subscribe` y se enviarán a la función de `query` correspondiente en el backend. Los argumentos deben ser un diccionario con claves de tipo cadena y, en general, los valores deben ser tipos primitivos, arrays y otros diccionarios.

```swift
let publisher = convex.subscribe(to: "colors:get",
                               with:["onlyFavorites": true],
                           yielding:[String].self)
```

Suponiendo que la consulta `colors:get` acepte un argumento `onlyFavorites`, el valor
se puede recibir y usar para aplicar lógica en la función de consulta.

<Admonition type="tip">
  Usa [structs Decodable](/client/swift/data-types.md#custom-data-types)
  para convertir automáticamente objetos de Convex en structs de Swift.
</Admonition>

<Admonition type="caution">
  * Hay detalles importantes a tener en cuenta al
    [enviar y recibir números](/client/swift/data-types.md#numerical-types)
    entre Swift y Convex.
  * Dependiendo de tus funciones de backend, es posible que debas gestionar
    [palabras clave reservadas de Swift](/client/swift/data-types.md#field-name-conversion).
</Admonition>

### Duración de la suscripción \{#subscription-lifetime\}

El `Publisher` devuelto por `subscribe` permanecerá activo mientras exista el
`View` o `ObservableObject` asociado. Cuando cualquiera de ellos deje de formar parte de la interfaz de usuario, se cancelará la suscripción a la consulta subyacente en Convex.

## Editar datos \{#editing-data\}

Puedes usar el método `mutation` en `ConvexClient` para ejecutar una
[mutación](/functions/mutation-functions.mdx) en el backend.

`mutation` es un método `async`, por lo que tendrás que llamarlo dentro de una `Task`.
Las mutaciones pueden devolver un valor o no.

Las mutaciones también pueden recibir argumentos, igual que las consultas. Aquí tienes un ejemplo de
cómo llamar a una mutación con argumentos que devuelve un valor:

```swift
let isColorAdded: Bool = try await convex.mutation("colors:put", with: ["color": newColor])
```

### Manejo de errores \{#handling-errors\}

Si se produce un error durante una llamada a `mutation`, se lanzará una excepción. Normalmente querrás
capturar [`ConvexError`](/functions/error-handling/application-errors.mdx) y `ServerError` y
gestionarlos de la forma que sea apropiada en tu aplicación.

Aquí tienes un pequeño ejemplo de cómo podrías manejar un error de `colors:put` si
lanzara un `ConvexError` con un mensaje de error indicando que el color ya existe.

```swift
do {
  try await convex.mutation("colors:put", with: ["color": newColor])
} catch ClientError.ConvexError(let data) {
  errorMessage = try! JSONDecoder().decode(String.self, from: Data(data.utf8))
  colorNotAdded = true
}
```

Consulta la documentación sobre [gestión de errores](/functions/error-handling/) para obtener más información.

## Llamar a APIs de terceros \{#calling-third-party-apis\}

Puedes usar el método `action` en `ConvexClient` para invocar una
[action](/functions/actions.mdx) en el backend.

Las llamadas a `action` pueden aceptar argumentos, devolver valores y lanzar excepciones igual que las llamadas a `mutation`.

Aunque puedes llamar a acciones desde tu código de cliente, no siempre es la opción correcta. Consulta la documentación de acciones para obtener consejos sobre
[llamar a acciones desde clientes](/functions/actions.mdx#calling-actions-from-clients).

## Autenticación con Auth0 \{#authentication-with-auth0\}

Puedes usar `ConvexClientWithAuth` en lugar de `ConvexClient` para configurar
la autenticación con [Auth0](https://auth0.com/). Necesitarás
la librería `convex-swift-auth0` para hacerlo, así como una cuenta de Auth0 y
la configuración de la aplicación.

Consulta
el [README](https://github.com/get-convex/convex-swift-auth0/blob/main/README.md) en
el repositorio `convex-swift-auth0` para obtener instrucciones de configuración más detalladas, y
la [aplicación de ejemplo Workout](https://github.com/get-convex/ios-convex-workout), que
está configurada para Auth0. La [documentación de autenticación de Convex](/auth.mdx) en general
también es un buen recurso.

También debería ser posible integrar otros proveedores de autenticación similares basados en OpenID Connect.
Consulta
el protocolo [`AuthProvider`](https://github.com/get-convex/convex-swift/blob/c47aea414c92db2ccf3a0fa4f9db8caf2029b032/Sources/ConvexMobile/ConvexMobile.swift#L188)
en el repositorio `convex-swift` para más información.

## Despliegues de producción y dev \{#production-and-dev-deployments\}

Cuando estés listo para avanzar hacia [producción](/production.mdx) para tu app,
puedes configurar tu sistema de compilación de Xcode para que distintos
destinos de compilación apunten a distintos despliegues de Convex. La
configuración del entorno de compilación es muy específica, y es posible que tú
o tu equipo tengan convenciones diferentes, pero esta es una forma de abordar
el problema.

1. Crea carpetas “Dev” y “Prod” en el código fuente de tu proyecto.
2. Agrega un archivo `Env.swift` en cada una con contenido como:

```swift
let deploymentUrl = "https://$DEV_OR_PROD.convex.cloud"
```

3. Coloca tu URL de dev en `Dev/Env.swift` y tu URL de prod en `Prod/Env.swift`.
   No te preocupes si Xcode muestra un aviso de que `deploymentUrl` está definido varias
   veces.
4. Haz clic en tu proyecto de nivel superior en la vista del explorador de la izquierda.
5. Selecciona tu destino de compilación en la lista **TARGETS**.
6. Cambia el nombre del destino para que termine en “dev”.
7. Haz clic derecho/Ctrl-clic sobre él y duplícalo, dándole un nombre que termine en “prod”.
8. Con el destino “dev” seleccionado, haz clic en la pestaña **Build Phases**.
9. Expande la sección **Compile Sources**.
10. Selecciona `Prod/Env.swift` y elimínalo con el botón -.
11. Del mismo modo, abre el destino “prod” y elimina `Dev/Env.swift` de sus
    archivos de origen.

![Screenshot 2024-10-03 at 1.34.34 PM.png](/screenshots/swift_env_setup.png)

Ahora puedes hacer referencia a `deploymentUrl` dondequiera que crees tu `ConvexClient` y,
según el destino que compiles, utilizará tu URL de dev o de prod.

## Estructurar tu aplicación \{#structuring-your-application\}

Los ejemplos mostrados en esta guía están pensados para ser breves y no
ofrecen orientación sobre cómo estructurar una aplicación completa.

Si quieres un enfoque más sólido y por capas, coloca el código que interactúa
con `ConvexClient` en una clase que cumpla con `ObservableObject`. Luego tu
`View` puede observar ese objeto como un `@StateObject` y se reconstruirá
cada vez que cambie.

Por ejemplo, si adaptamos el ejemplo `colors:get` de arriba a una clase
`ViewModel: ObservableObject`, la `View` ya no participa directamente en
la obtención de los datos: solo sabe que la lista de `colors` la proporciona el
`ViewModel`.

```swift
import SwiftUI

class ViewModel: ObservableObject {
  @Published var colors: [String] = []

  init() {
    convex.subscribe(to: "colors:get")
      .replaceError(with: [])
      .receive(on: DispatchQueue.main)
      .assign(to: &$colors)
  }
}

struct ContentView: View {
  @StateObject var viewModel = ViewModel()

  var body: some View {
    List {
      ForEach(viewModel.colors, id: \.self) { color in
        Text(color)
      }
    }
  }
}
```

Dependiendo de tus necesidades y de la escala de tu aplicación, puede tener sentido darle
una estructura aún más formal, tal y como se muestra en, por ejemplo,
https://github.com/nalexn/clean-architecture-swiftui.

## Bajo el capó \{#under-the-hood\}

La biblioteca de Convex para Swift está construida sobre el
[cliente oficial de Convex para Rust](/client/rust.md). Se encarga de mantener una
conexión WebSocket con el backend de Convex e implementa el protocolo completo
de Convex.

Todas las llamadas a métodos de `ConvexClient` se gestionan mediante el entorno
de ejecución asíncrono Tokio en el lado de Rust y es seguro realizarlas desde
el actor principal de la aplicación.