---
title: "OCC y atomicidad"
slug: "occ"
hidden: false
sidebar_position: 500
todo: Push under mutations, or inline, or kill (move to Stack)
description:
  "Control de concurrencia optimista y atomicidad de transacciones en Convex"
---

En [Consultas](/functions/query-functions.mdx), mencionamos que el determinismo
era importante en la forma en que se utilizaba el control de concurrencia
optimista (OCC) dentro de Convex. En esta sección, profundizaremos mucho más en
*el por qué*.

## Convex Financial, Inc. \{#convex-financial-inc\}

Imagina que estás creando una aplicación bancaria y, por lo tanto, tu base de datos almacena
cuentas con saldos. Quieres que tus usuarios puedan enviarse dinero entre sí,
así que escribes una función de mutación que transfiere fondos de la cuenta de un usuario
a la de otro.

Una ejecución de esa transacción podría leer primero el saldo de la cuenta de Alice y luego el de Bob.
Entonces planteas descontar 5 $ de la cuenta de Alice y aumentar el saldo de Bob
en los mismos 5 $.

Aquí está nuestro pseudocódigo:

```
$14 <- READ Alice
$11 <- READ Bob
WRITE Alice $9
WRITE Bob $16
```

Esta transferencia de saldo en el libro mayor es un escenario clásico de bases de datos que requiere
garantizar que estas operaciones de escritura solo se apliquen de forma conjunta. ¡Es un problema serio
si solo una de las operaciones se completa correctamente!

```
$14 <- READ Alice
$11 <- READ Bob
WRITE Alice $9
*crash* // Se perdieron $5 de tu banco
```

Necesitas una garantía de que esto nunca pueda suceder. Requieres atomicidad
en las transacciones, y Convex la proporciona.

El problema de la integridad de los datos es mucho más profundo. Las transacciones
concurrentes que leen y editan los mismos registros pueden crear *condiciones de carrera de datos*.

En el caso de nuestra app, es totalmente posible que alguien descuente el saldo
de Alice justo después de que lo leamos. Tal vez compró una Coke Zero en el
aeropuerto con su tarjeta de débito por $3.

```
$5 Transfer                           $3 Debit Card Charge
----------------------------------------------------------
$14 <- READ Alice
$11 <- READ Bob
                                        $14 <- READ Alice
                                        WRITE Alice $11
WRITE Alice $9 // Free coke!
WRITE Bob $16
```

Claramente, necesitamos evitar que se produzcan este tipo de condiciones de carrera de datos. Necesitamos una
forma de manejar estos conflictos concurrentes. En general, hay dos enfoques
comunes.

La mayoría de las bases de datos tradicionales eligen una estrategia de *bloqueo pesimista*. (Pesimismo,
en este caso, significa que la estrategia asume que el conflicto ocurrirá de antemano y
procura prevenirlo.) Con el bloqueo pesimista, primero necesitas adquirir un bloqueo
en el registro de Alice y luego adquirir un bloqueo en el registro de Bob. Entonces puedes proceder
a realizar tu transacción, sabiendo que cualquier otra transacción que necesite
tocar esos registros esperará hasta que termines y todas tus escrituras estén
confirmadas.

Después de décadas de experiencia, las desventajas del bloqueo pesimista están bien
conocidas y son innegables. La mayor limitación surge de que las redes y las
computadoras reales son inherentemente poco confiables. Si el poseedor del bloqueo desaparece por
cualquier motivo a mitad de su transacción, todos los demás que quieran
modificar cualquiera de esos registros estarán esperando indefinidamente. ¡Nada bueno!

El control de concurrencia optimista es, como dice el nombre, optimista. Asume
que la transacción tendrá éxito y no se preocupa por bloquear nada por adelantado.
¡Muy atrevido! ¿Cómo puede estar tan seguro?

Lo hace tratando la transacción como una *propuesta declarativa* para escribir
registros basándose en las versiones de los registros que haya leído (el «conjunto de lectura»). Al final de
la transacción, todas las escrituras se confirman si cada versión en el conjunto de lectura sigue siendo
la versión más reciente de ese registro. Esto significa que no ocurrió ningún conflicto concurrente.

Ahora, usando nuestro conjunto de lectura de versiones, veamos cómo OCC habría evitado la
catástrofe del refresco de arriba:

```
$5 Transfer                           $3 Debit Card Charge
----------------------------------------------------------
(v1, $14) <- READ Alice
(v7, $11) <- READ Bob
                                        (v1, $14) <- READ Alice
                                        WRITE Alice $11
                                        IF Alice.v = v1

WRITE Alice = $9, Bob = $16
    IF Alice.v = v1, Bob.v = v7 // ¡Falla! Alice = v2
```

Esto es parecido a no poder hacer push a tu repositorio de Git porque no estás en
HEAD. Todos sabemos que, en esa situación, hay que hacer pull y luego rebasear o hacer merge,
etc.

## Cuando OCC pierde, gana el determinismo \{#when-occ-loses-determinism-wins\}

Una solución ingenua de control de concurrencia optimista sería resolver esto de
la misma manera que lo hace Git: exigir al usuario/aplicación que resuelva el
conflicto y determine si es seguro reintentar.

En Convex, sin embargo, no necesitamos hacer eso. Sabemos que la transacción es
determinista. No realizó un cargo en Stripe, no escribió un valor permanente en
el sistema de archivos. No tuvo ningún efecto en absoluto aparte de proponer
algunos cambios atómicos en las tablas de Convex que no se aplicaron.

El determinismo significa que simplemente podemos volver a ejecutar la
transacción; nunca necesitas preocuparte por condiciones de carrera temporales.
Podemos ejecutar varios reintentos si es necesario hasta que logremos ejecutar
la transacción sin ningún conflicto.

<Admonition type="tip">
  De hecho, la analogía con Git sigue siendo muy acertada. Un conflicto de OCC
  significa que no podemos hacer push porque nuestro HEAD está desactualizado, así
  que necesitamos hacer rebase de nuestros cambios e intentarlo de nuevo. Y el
  determinismo es lo que garantiza que nunca haya un &quot;merge conflict&quot;, por lo que
  (a diferencia de Git) esta operación de rebase siempre terminará teniendo éxito
  sin intervención de la persona desarrolladora.
</Admonition>

## Aislamiento por instantánea vs Serializabilidad \{#snapshot-isolation-vs-serializability\}

Es común que las bases de datos con control de concurrencia optimista multiversión
proporcionen la garantía de
[aislamiento por instantánea](https://en.wikipedia.org/wiki/Snapshot_isolation). Este
[nivel de aislamiento](https://en.wikipedia.org/wiki/Isolation_\(database_systems\))
proporciona la ilusión de que todas las transacciones se ejecutan sobre una instantánea atómica de los
datos, pero es vulnerable a
[anomalías](https://en.wikipedia.org/wiki/Snapshot_isolation#Definition) en las que
ciertas combinaciones de transacciones concurrentes pueden producir resultados incorrectos. La
implementación del control de concurrencia optimista en Convex, en cambio, proporciona una verdadera
[serializabilidad](https://en.wikipedia.org/wiki/Serializability) y producirá
resultados correctos independientemente de qué transacciones se emitan de forma concurrente.

## No tienes que pensar en esto \{#no-need-to-think-about-this\}

La belleza de este enfoque es que puedes simplemente escribir tus funciones de mutación
como si *siempre fueran a tener éxito*, con la garantía de que siempre serán atómicas.

Más allá de la simple curiosidad sobre cómo funciona Convex, en el día a día no necesitas
preocuparte por conflictos, bloqueos ni atomicidad cuando haces cambios en tus
tablas y documentos. La forma &quot;obvia&quot; de escribir tus funciones de mutación
simplemente funcionará.