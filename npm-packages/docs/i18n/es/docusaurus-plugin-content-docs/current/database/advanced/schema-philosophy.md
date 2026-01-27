---
title: Filosofía del esquema
sidebar_position: 450
description: "Filosofía de diseño de esquemas en Convex y buenas prácticas"
---

Con Convex no necesitas escribir ninguna instrucción `CREATE TABLE`, ni pensar
de antemano en la estructura de tus tablas almacenadas para poder nombrar
tus campos y tipos. Simplemente guardas tus objetos en Convex y sigues
construyendo tu aplicación.

Sin embargo, avanzar muy rápido al principio puede ser problemático más adelante.
"¿Ese campo era un número o una cadena? ¿Creo que lo cambié cuando arreglé
ese bug?"

Los sistemas de almacenamiento que son demasiado permisivos a veces pueden
convertirse en una carga a medida que tu sistema madura y quieres poder razonar
con seguridad sobre exactamente qué datos hay en tu sistema.

La buena noticia es que Convex siempre está tipado. ¡Solo que está tipado de
forma implícita! Cuando envías un documento a Convex, este rastrea todos los
tipos de todos los campos de tu documento. Puedes ir a tu
[panel de control](/dashboard.md) y ver el esquema inferido de cualquier tabla
para entender qué es lo que tienes.

"¿Qué pasa con ese campo que cambié de cadena a número?" Convex también puede
manejar esto. Convex rastreará esos cambios; en este caso, el campo es una unión
como `v.union(v.number(), v.string())`. De ese modo, incluso cuando cambias de
opinión sobre los campos y tipos de tus documentos, Convex te cubre las espaldas.

Cuando estés listo para formalizar tu esquema, puedes definirlo usando nuestro
[constructor de esquemas](/database/schemas.mdx) para habilitar la validación
del esquema y generar tipos basados en él.