---
title: "Transmisión de datos hacia y desde Convex"
sidebar_label: "Importación/exportación en streaming"
description: "Transmisión de datos hacia y desde Convex"
sidebar_position: 4
---

[Fivetran](https://www.fivetran.com) y [Airbyte](https://airbyte.com) son
plataformas de integración de datos que te permiten sincronizar tus datos de
Convex con otras bases de datos.

Fivetran permite la exportación en streaming desde Convex a cualquiera de sus
[destinos compatibles](https://fivetran.com/docs/destinations). El equipo de
Convex mantiene un conector de origen de Convex para exportación en streaming.
La importación en streaming en Convex mediante Fivetran no está disponible por
el momento.

El uso de Airbyte permite la importación en streaming desde cualquiera de sus
[orígenes compatibles](https://airbyte.com/connectors?connector-type=Sources)
hacia Convex y la exportación en streaming desde Convex hacia cualquiera de sus
[destinos compatibles](https://airbyte.com/connectors?connector-type=Destinations).
El equipo de Convex mantiene un conector de origen de Convex para exportación en
streaming y un conector de destino de Convex para importación en streaming.

<BetaAdmonition feature="integraciones de Fivetran y Airbyte" verb="son" />

## Exportación en streaming \{#streaming-export\}

La exportación de datos puede ser útil para manejar cargas de trabajo que Convex
no admite directamente. Algunos casos de uso incluyen:

1. Analítica
   * Convex no está optimizado para consultas que cargan cantidades enormes de
     datos. Una plataforma de datos como [Databricks](https://www.databricks.com) o
     [Snowflake](https://www.snowflake.com/) es más apropiada.
2. Consultas flexibles
   * Aunque Convex cuenta con potentes
     [consultas de base de datos](/database/reading-data/reading-data.mdx#querying-documents)
     y compatibilidad integrada con la [búsqueda de texto completo](/search.mdx),
     siguen existiendo algunas consultas que son difíciles de escribir dentro de
     Convex. Si necesitas un ordenamiento y filtrado muy dinámicos para algo como una vista de &quot;búsqueda avanzada&quot;,
     bases de datos como [ElasticSearch](https://www.elastic.co) pueden ser útiles.
3. Entrenamiento de machine learning
   * Convex no está optimizado para consultas que ejecutan algoritmos de
     machine learning con un uso intensivo de cómputo.

<ProFeatureUpsell feature="Streaming export" verb="requires" />

Consulta la documentación de [Fivetran](https://fivetran.com/integrations/convex) o
[Airbyte](https://docs.airbyte.com/integrations/sources/convex) para saber
cómo configurar una exportación en streaming. [Contáctanos](https://convex.dev/community) si
necesitas ayuda o tienes preguntas.

## Importación por streaming \{#streaming-import\}

Adoptar nuevas tecnologías puede ser un proceso lento y abrumador, especialmente cuando
implican bases de datos. La importación por streaming permite adoptar Convex
junto con tu stack existente sin tener que escribir tus propias herramientas de migración o de sincronización
de datos. Algunos casos de uso incluyen:

1. Prototipar cómo Convex podría reemplazar el backend existente de tu proyecto usando
   sus propios datos.
2. Crear productos nuevos más rápido usando Convex junto con bases de datos existentes.
3. Desarrollar una capa de UI reactiva sobre un conjunto de datos existente.
4. Migrar tus datos a Convex (si la herramienta de la [CLI](/cli.md) no satisface tus
   necesidades).

<Admonition type="caution" title="Marca las tablas importadas como de solo lectura">
  Un caso de uso común es &quot;replicar&quot; una tabla en la base de datos de origen en Convex para
  crear algo nuevo usando Convex. Recomendamos dejar las tablas importadas
  como de solo lectura en Convex porque sincronizar los resultados de vuelta a la base de datos
  de origen podría producir conflictos de escritura peligrosos. Aunque Convex todavía no
  tiene controles de acceso que garanticen que una tabla sea de solo lectura, puedes asegurarte de que
  no haya mutaciones ni acciones que escriban en tablas importadas en tu código y evitar editar
  documentos en tablas importadas en el panel de control.
</Admonition>

La importación por streaming está incluida en todos los planes de Convex. Consulta la documentación de Airbyte sobre cómo
configurar el conector de destino de Convex
[aquí](https://docs.airbyte.com/integrations/destinations/convex).