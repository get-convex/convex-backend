---
title: "Conversión de tipos entre Swift y Convex"
sidebar_label: "Tipos de datos"
hidden: false
sidebar_position: 5
description: "Personalización y conversión de tipos entre la aplicación Swift y Convex"
---

## Tipos de datos personalizados \{#custom-data-types\}

Convex te permite expresar fácilmente tus datos en el backend como objetos de TypeScript,
y puede devolver esos objetos desde consultas, mutaciones y acciones. Para manejar
objetos en Swift, crea definiciones de `struct` que implementen el
protocolo `Decodable`. Por lo general esto es bastante sencillo de hacer, ya que cualquier `struct` cuyos
miembros sean todos `Decodable` puede adoptarlo automáticamente.

Considera una función de consulta de Convex que devuelve resultados como este objeto de JavaScript:

```tsx
{
  name: "Guardians",
  uniformColors: ["blue", "white", "red"],
  wins: 80n,
  losses: 60n
}
```

Esto se puede representar en Swift como:

```swift
struct BaseballTeam: Decodable {
  let name: String
  let uniformColors: [String]
  @ConvexInt
  var wins: Int
  @ConvexInt
  var losses: Int
}
```

Luego puedes pasar ese tipo como el argumento que se emite en tu llamada a subscribe:

```swift
convex.subscribe(to: "mlb:first_place_team",
               with: ["division": "AL Central"],
           yielding: BaseballTeam.self)
```

Los datos de la función remota se deserializarán en tu `struct` personalizado.
A menudo, el tipo que usas puede inferirse a partir del contexto de la llamada y puedes
omitir el argumento `yielding`.

## Tipos numéricos \{#numerical-types\}

Los tipos numéricos como `Int` y `Double` se codifican en un formato especial para garantizar
la interoperabilidad adecuada con tus funciones backend de TypeScript. Para usarlos de forma segura
en el lado de Swift, asegúrate de usar uno de los siguientes wrappers de propiedades.

| Tipo                           | Wrapper                |
| ------------------------------ | ---------------------- |
| `Float` o `Double`             | `@ConvexFloat`         |
| `Float?` o `Double?`           | `@OptionalConvexFloat` |
| `Int` o `Int32` o `Int64`      | `@ConvexInt`           |
| `Int?` o `Int32?` o `Int64?`   | `@OptionalConvexInt`   |

Ten en cuenta que las propiedades de `struct` con wrappers de propiedades deben declararse como `var`.

## Conversión de nombres de campos \{#field-name-conversion\}

Si tu código recibe objetos con nombres que necesitas o quieres convertir
a otros nombres, puedes usar un `enum` `CodingKeys` para especificar un mapa de
nombres remotos a nombres en tu `struct`. Por ejemplo, imagina una función de backend o
una API que devuelve entradas de registro como las siguientes, que representan cuándo alguien entró
y salió:

```tsx
{name: "Bob", in: "2024-10-03 08:00:00", out: "2024-10-03 11:00:00"}
```

Esos datos no se pueden decodificar directamente en una `struct` porque `in` es una palabra clave en Swift. Podemos usar `CodingKeys` para darle un nombre alternativo y aun así seguir leyendo los datos usando el nombre original.

```swift
struct Log: Decodable {
  let name: String
  let inTime: String
  let outTime: String

  enum CodingKeys: String, CodingKey {
    case name
    case inTime = "in"
    case outTime = "out"
  }
}
```

## Poniéndolo todo junto \{#putting-it-all-together\}

En el ejemplo de tipo de datos personalizado anterior, se usa el tipo `BigInt` de JavaScript en los datos del backend agregando una `n` al final de los valores de `wins` y `losses`, lo que permite que el código Swift use `Int`. Si, en cambio, el código usara tipos `number` convencionales de JavaScript, del lado de Swift se recibirían como valores de punto flotante y la deserialización a `Int` fallaría.

Si te encuentras en una situación como esa en la que se usa `number` pero, por convención, solo contiene valores enteros, puedes manejarlo en tu `struct` usando conversión de nombres de campos y propiedades personalizadas para ocultar la representación en punto flotante.

```swift
struct BaseballTeam: Decodable {
  let name: String
  let uniformColors: [String]
  @ConvexFloat
  private var internalWins: Double
  @ConvexFloat
  private var internalLosses: Double

  enum CodingKeys: String, CodingKey {
    case name
    case uniformColors
    case internalWins = "wins"
    case internalLosses = "losses"
  }

  // Expone los valores Double como Ints
  var wins: Int { Int(internalWins) }
  var losses: Int { Int(internalLosses) }
}
```

El patrón consiste en almacenar los valores de `Double` de forma privada y con nombres
distintos del valor que proviene del backend. Luego agrega propiedades personalizadas para proporcionar
los valores de `Int`.
