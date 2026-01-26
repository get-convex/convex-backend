---
id: "server"
title: "Módulo: server"
custom_edit_url: null
---

Utilidades para implementar funciones de consulta y de mutación de Convex en el servidor.

## Uso \{#usage\}

### Generación de código \{#code-generation\}

Este módulo suele usarse junto con el código de servidor generado.

Para generar el código del servidor, ejecuta `npx convex dev` en tu proyecto de Convex.
Esto creará un archivo `convex/_generated/server.js` con las siguientes
funciones, con tipos definidos para tu esquema:

* [query](https://docs.convex.dev/generated-api/server#query)
* [mutation](https://docs.convex.dev/generated-api/server#mutation)

Si no estás usando TypeScript ni la generación de código, puedes usar en su lugar estas
funciones sin tipos:

* [queryGeneric](server.md#querygeneric)
* [mutationGeneric](server.md#mutationgeneric)

### Ejemplo \{#example\}

Las funciones de Convex se definen mediante los *wrappers* `query` o
`mutation`.

Las consultas reciben un objeto `db` que implementa la interfaz [GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md).

```js
import { query } from "./_generated/server";

export default query({
  handler: async ({ db }, { arg1, arg2 }) => {
    // ¡Tu código (de solo lectura) aquí!
  },
});
```

Si tu función necesita escribir en la base de datos, por ejemplo para insertar, actualizar
o eliminar documentos, usa en su lugar `mutation`, que proporciona un `db` que
implementa la interfaz [GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md).

```js
import { mutation } from "./_generated/server";

export default mutation({
  handler: async ({ db }, { arg1, arg2 }) => {
    // ¡Tu código de mutación aquí!
  },
});
```

## Clases \{#classes\}

* [Crons](../classes/server.Crons.md)
* [Expression](../classes/server.Expression.md)
* [IndexRange](../classes/server.IndexRange.md)
* [HttpRouter](../classes/server.HttpRouter.md)
* [TableDefinition](../classes/server.TableDefinition.md)
* [SchemaDefinition](../classes/server.SchemaDefinition.md)
* [SearchFilter](../classes/server.SearchFilter.md)
* [FilterExpression](../classes/server.FilterExpression.md)

## Interfaces \{#interfaces\}

* [UserIdentity](../interfaces/server.UserIdentity.md)
* [Auth](../interfaces/server.Auth.md)
* [CronJob](../interfaces/server.CronJob.md)
* [BaseTableReader](../interfaces/server.BaseTableReader.md)
* [GenericDatabaseReader](../interfaces/server.GenericDatabaseReader.md)
* [GenericDatabaseReaderWithTable](../interfaces/server.GenericDatabaseReaderWithTable.md)
* [GenericDatabaseWriter](../interfaces/server.GenericDatabaseWriter.md)
* [GenericDatabaseWriterWithTable](../interfaces/server.GenericDatabaseWriterWithTable.md)
* [BaseTableWriter](../interfaces/server.BaseTableWriter.md)
* [FilterBuilder](../interfaces/server.FilterBuilder.md)
* [IndexRangeBuilder](../interfaces/server.IndexRangeBuilder.md)
* [PaginationResult](../interfaces/server.PaginationResult.md)
* [PaginationOptions](../interfaces/server.PaginationOptions.md)
* [QueryInitializer](../interfaces/server.QueryInitializer.md)
* [Query](../interfaces/server.Query.md)
* [OrderedQuery](../interfaces/server.OrderedQuery.md)
* [GenericMutationCtx](../interfaces/server.GenericMutationCtx.md)
* [GenericQueryCtx](../interfaces/server.GenericQueryCtx.md)
* [GenericActionCtx](../interfaces/server.GenericActionCtx.md)
* [ValidatedFunction](../interfaces/server.ValidatedFunction.md)
* [Scheduler](../interfaces/server.Scheduler.md)
* [SearchIndexConfig](../interfaces/server.SearchIndexConfig.md)
* [VectorIndexConfig](../interfaces/server.VectorIndexConfig.md)
* [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md)
* [SystemDataModel](../interfaces/server.SystemDataModel.md)
* [SearchFilterBuilder](../interfaces/server.SearchFilterBuilder.md)
* [SearchFilterFinalizer](../interfaces/server.SearchFilterFinalizer.md)
* [StorageReader](../interfaces/server.StorageReader.md)
* [StorageWriter](../interfaces/server.StorageWriter.md)
* [StorageActionWriter](../interfaces/server.StorageActionWriter.md)
* [VectorSearchQuery](../interfaces/server.VectorSearchQuery.md)
* [VectorFilterBuilder](../interfaces/server.VectorFilterBuilder.md)

## Referencias \{#references\}

### UserIdentityAttributes \{#useridentityattributes\}

Vuelve a exportar [UserIdentityAttributes](browser.md#useridentityattributes)

## Alias de tipos \{#type-aliases\}

### FunctionType \{#functiontype\}

Ƭ **FunctionType**: `"query"` | `"mutation"` | `"action"`

El tipo de función de Convex.

#### Definido en \{#defined-in\}

[server/api.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L19)

***

### FunctionReference \{#functionreference\}

Ƭ **FunctionReference**&lt;`Type`, `Visibility`, `Args`, `ReturnType`, `ComponentPath`&gt;: `Object`

Una referencia a una función registrada de Convex.

Puedes crear un [FunctionReference](server.md#functionreference) mediante la utilidad `api` generada:

```js
import { api } from "../convex/_generated/api";

const reference = api.myModule.myFunction;
```

Si no utilizas la generación de código, puedes crear referencias con
[anyApi](server.md#anyapi-1):

```js
import { anyApi } from "convex/server";

const reference = anyApi.myModule.myFunction;
```

Las referencias de funciones se pueden usar para invocar funciones desde el cliente. Por ejemplo, en React puedes pasar referencias al hook [useQuery](react.md#usequery):

```js
const result = useQuery(api.myModule.myFunction);
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) | El tipo de la función (&quot;query&quot;, &quot;mutation&quot; o &quot;action&quot;). |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) = `"public"` | La visibilidad de la función (&quot;public&quot; o &quot;internal&quot;). |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` | Los argumentos de esta función. Es un objeto que asigna los nombres de los argumentos a sus tipos. |
| `ReturnType` | `any` | El tipo de retorno de esta función. |
| `ComponentPath` | `string` | `undefined` | - |

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `_type` | `Type` |
| `_visibility` | `Visibility` |
| `_args` | `Args` |
| `_returnType` | `ReturnType` |
| `_componentPath` | `ComponentPath` |

#### Definido en \{#defined-in\}

[server/api.ts:52](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L52)

***

### ApiFromModules \{#apifrommodules\}

Ƭ **ApiFromModules**&lt;`AllModules`&gt;: [`FilterApi`](server.md#filterapi)&lt;`ApiFromModulesAllowEmptyNodes`&lt;`AllModules`&gt;, [`FunctionReference`](server.md#functionreference)&lt;`any`, `any`, `any`, `any`&gt;&gt;

Dados los tipos de todos los módulos en el directorio `convex/`, se construye el tipo
de `api`.

`api` es una utilidad para construir [FunctionReference](server.md#functionreference).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `AllModules` | extends `Record`&lt;`string`, `object`&gt; | Un tipo que asocia rutas de módulos (como `"dir/myModule"`) con los tipos de esos módulos. |

#### Definido en \{#defined-in\}

[server/api.ts:255](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L255)

***

### FilterApi \{#filterapi\}

Ƭ **FilterApi**&lt;`API`, `Predicate`&gt;: [`Expand`](server.md#expand)&lt;&#123; [mod in keyof API as API[mod] extends Predicate ? mod : API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? never : FilterApi&lt;API[mod], Predicate&gt; extends Record&lt;string, never&gt; ? never : mod]: API[mod] extends Predicate ? API[mod] : FilterApi&lt;API[mod], Predicate&gt; &#125;&gt;

Filtra el objeto de API de un despliegue de Convex para obtener las funciones que cumplan ciertos criterios,
por ejemplo todas las consultas públicas.

#### Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `API` |
| `Predicate` |

#### Definido en \{#defined-in\}

[server/api.ts:279](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L279)

***

### AnyApi \{#anyapi\}

Ƭ **AnyApi**: `Record`&lt;`string`, `Record`&lt;`string`, `AnyModuleDirOrFunc`&gt;&gt;

El tipo que extienden los objetos de API de Convex. Si escribieras una API desde cero, debería extender este tipo.

#### Definido en \{#defined-in\}

[server/api.ts:393](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L393)

***

### PartialApi \{#partialapi\}

Ƭ **PartialApi**&lt;`API`&gt;: &#123; [mod in keyof API]?: API[mod] extends FunctionReference&lt;any, any, any, any&gt; ? API[mod] : PartialApi&lt;API[mod]&gt; &#125;

API parcial recursiva, útil para definir un subconjunto de una API al crear mocks
o al construir objetos de API personalizados.

#### Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `API` |

#### Definido en \{#defined-in\}

[server/api.ts:401](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L401)

***

### FunctionArgs \{#functionargs\}

Ƭ **FunctionArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`]

Dado un [FunctionReference](server.md#functionreference), obtiene el tipo de retorno de la función.

Se representa como un objeto que asocia nombres de argumentos con valores.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### Definido en \{#defined-in\}

[server/api.ts:435](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L435)

***

### OptionalRestArgs \{#optionalrestargs\}

Ƭ **OptionalRestArgs**&lt;`FuncRef`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject] : [args: FuncRef[&quot;&#95;args&quot;]]

Un tipo de tupla de los argumentos (posiblemente opcionales) de `FuncRef`.

Este tipo se usa para que los métodos que involucran argumentos sean seguros en cuanto a tipos, permitiendo al mismo tiempo
omitir los argumentos en las funciones que no los requieren.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### Definido en \{#defined-in\}

[server/api.ts:446](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L446)

***

### ArgsAndOptions \{#argsandoptions\}

Ƭ **ArgsAndOptions**&lt;`FuncRef`, `Options`&gt;: `FuncRef`[`"_args"`] extends `EmptyObject` ? [args?: EmptyObject, options?: Options] : [args: FuncRef[&quot;&#95;args&quot;], options?: Options]

Un tipo de tupla formada por los argumentos (posiblemente opcionales) de `FuncRef`, seguidos de un objeto de opciones de tipo `Options`.

Este tipo se usa para que métodos como `useQuery` tengan tipado seguro, a la vez que permiten:

1. Omitir los argumentos para funciones que no los requieren.
2. Omitir el objeto de opciones.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |
| `Options` | `Options` |

#### Definido en \{#defined-in\}

[server/api.ts:460](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L460)

***

### FunctionReturnType \{#functionreturntype\}

Ƭ **FunctionReturnType**&lt;`FuncRef`&gt;: `FuncRef`[`"_returnType"`]

Dado un [FunctionReference](server.md#functionreference), devuelve el tipo de retorno de la función.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `FuncRef` | extends `AnyFunctionReference` |

#### Definido en \{#defined-in\}

[server/api.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L472)

***

### AuthConfig \{#authconfig\}

Ƭ **AuthConfig**: `Object`

El valor exportado por tu proyecto Convex en `auth.config.ts`.

```ts
import { AuthConfig } from "convex/server";

export default {
  providers: [
    {
      domain: "https://your.issuer.url.com",
      applicationID: "your-application-id",
    },
  ],
} satisfies AuthConfig;
```

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `providers` | [`AuthProvider`](server.md#authprovider)[] |

#### Definido en \{#defined-in\}

[server/authentication.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L19)

***

### AuthProvider \{#authprovider\}

Ƭ **AuthProvider**: &#123; `applicationID`: `string` ; `domain`: `string`  &#125; | &#123; `type`: `"customJwt"` ; `applicationID?`: `string` ; `issuer`: `string` ; `jwks`: `string` ; `algorithm`: `"RS256"` | `"ES256"`  &#125;

Un proveedor de autenticación autorizado para emitir JWTs para tu aplicación.

Consulta: https://docs.convex.dev/auth/advanced/custom-auth y https://docs.convex.dev/auth/advanced/custom-jwt

#### Definido en \{#defined-in\}

[server/authentication.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/authentication.ts#L28)

***

### FunctionHandle \{#functionhandle\}

Ƭ **FunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;: `string` &amp; [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"internal"`, `Args`, `ReturnType`&gt;

Una referencia serializable a una función de Convex.
Pasar esta referencia a otro componente permite que ese componente llame a esta
función durante la ejecución de la función actual o en cualquier momento posterior.
Los handles de función se usan igual que las FunctionReferences `api.folder.function`,
por ejemplo, `ctx.scheduler.runAfter(0, functionReference, args)`.

Una referencia de función es estable entre envíos de código, pero es posible
que la función de Convex a la que se refiere ya no exista.

Esta es una característica de los componentes, que están en beta.
Esta API es inestable y puede cambiar en versiones posteriores.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ReturnType` | `any` |

#### Definido en \{#defined-in\}

[server/components/index.ts:35](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L35)

***

### ComponentDefinition \{#componentdefinition\}

Ƭ **ComponentDefinition**&lt;`Exports`&gt;: `Object`

Un objeto de este tipo debería ser la exportación predeterminada de un
archivo convex.config.ts en un directorio de definición de componentes.

Esta es una característica de los componentes, que están en beta.
Esta API es inestable y puede cambiar en versiones posteriores.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `use` | &lt;Definition&gt;(`definition`: `Definition`, `options?`: &#123; `name?`: `string`  &#125;) =&gt; `InstalledComponent`&lt;`Definition`&gt; | Instala un componente según la definición proporcionada en esta definición de componente. Recibe una definición de componente y un nombre opcional. Para las herramientas del editor, este método espera un [ComponentDefinition](server.md#componentdefinition), pero en tiempo de ejecución el objeto importado será un ImportedComponentDefinition |
| `__exports` | `Exports` | Propiedad interna solo de tipo que realiza el seguimiento de las exportaciones proporcionadas. **`Deprecated`** Esta es una propiedad solo de tipo, no la uses. |

#### Definido en \{#defined-in\}

[server/components/index.ts:84](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L84)

***

### AnyChildComponents \{#anychildcomponents\}

Ƭ **AnyChildComponents**: `Record`&lt;`string`, `AnyComponentReference`&gt;

#### Definido en \{#defined-in\}

[server/components/index.ts:414](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L414)

***

### AnyComponents \{#anycomponents\}

Ƭ **AnyComponents**: [`AnyChildComponents`](server.md#anychildcomponents)

#### Definido en \{#defined-in\}

[server/components/index.ts:454](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L454)

***

### GenericDocument \{#genericdocument\}

Ƭ **GenericDocument**: `Record`&lt;`string`, [`Value`](values.md#value)&gt;

Un documento almacenado en Convex.

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:9](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L9)

***

### GenericFieldPaths \{#genericfieldpaths\}

Ƭ **GenericFieldPaths**: `string`

Un tipo que describe todos los campos de un documento en una tabla.

Estos pueden ser nombres de campos (como &quot;name&quot;) o referencias a campos en
objetos anidados (como &quot;properties.name&quot;).

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L18)

***

### GenericIndexFields \{#genericindexfields\}

Ƭ **GenericIndexFields**: `string`[]

Tipo que describe los campos ordenados de un índice.

Estos pueden ser nombres de campos (como &quot;name&quot;) o referencias a campos de
objetos anidados (como &quot;properties.name&quot;).

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:29](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L29)

***

### GenericTableIndexes \{#generictableindexes\}

Ƭ **GenericTableIndexes**: `Record`&lt;`string`, [`GenericIndexFields`](server.md#genericindexfields)&gt;

Un tipo que describe los índices de una tabla.

Es un objeto que mapea cada nombre de índice a los campos que componen el índice.

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L37)

***

### GenericSearchIndexConfig \{#genericsearchindexconfig\}

Ƭ **GenericSearchIndexConfig**: `Object`

Tipo que describe la configuración de un índice de búsqueda.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `searchField` | `string` |
| `filterFields` | `string` |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:43](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L43)

***

### GenericTableSearchIndexes \{#generictablesearchindexes\}

Ƭ **GenericTableSearchIndexes**: `Record`&lt;`string`, [`GenericSearchIndexConfig`](server.md#genericsearchindexconfig)&gt;

Tipo que describe todos los índices de búsqueda de una tabla.

Es un objeto que asocia cada nombre de índice con la configuración correspondiente de ese índice.

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L54)

***

### GenericVectorIndexConfig \{#genericvectorindexconfig\}

Ƭ **GenericVectorIndexConfig**: `Object`

Tipo que describe la configuración de un índice vectorial.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `vectorField` | `string` |
| `dimensions` | `number` |
| `filterFields` | `string` |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:63](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L63)

***

### GenericTableVectorIndexes \{#generictablevectorindexes\}

Ƭ **GenericTableVectorIndexes**: `Record`&lt;`string`, [`GenericVectorIndexConfig`](server.md#genericvectorindexconfig)&gt;

Un tipo que describe todos los índices vectoriales de una tabla.

Este es un objeto que asocia cada nombre de índice con la configuración del índice.

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:75](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L75)

***

### FieldTypeFromFieldPath \{#fieldtypefromfieldpath\}

Ƭ **FieldTypeFromFieldPath**&lt;`Document`, `FieldPath`&gt;: [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; extends [`Value`](values.md#value) | `undefined` ? [`FieldTypeFromFieldPathInner`](server.md#fieldtypefromfieldpathinner)&lt;`Document`, `FieldPath`&gt; : [`Value`](values.md#value) | `undefined`

El tipo de un campo en un documento.

Ten en cuenta que este tipo admite tanto campos simples como &quot;name&quot; como campos anidados como
&quot;properties.name&quot;.

Si el campo no está presente en el documento, se considera `undefined`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](server.md#genericdocument) |
| `FieldPath` | extiende `string` |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:104](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L104)

***

### FieldTypeFromFieldPathInner \{#fieldtypefromfieldpathinner\}

Ƭ **FieldTypeFromFieldPathInner**&lt;`Document`, `FieldPath`&gt;: `FieldPath` extends `$&#123;infer First&#125;.$&#123;infer Second&#125;` ? `ValueFromUnion`&lt;`Document`, `First`, `Record`&lt;`never`, `never`&gt;&gt; extends infer FieldValue ? `FieldValue` extends [`GenericDocument`](server.md#genericdocument) ? [`FieldTypeFromFieldPath`](server.md#fieldtypefromfieldpath)&lt;`FieldValue`, `Second`&gt; : `undefined` : `undefined` : `ValueFromUnion`&lt;`Document`, `FieldPath`, `undefined`&gt;

El tipo interno de [FieldTypeFromFieldPath](server.md#fieldtypefromfieldpath).

Está envuelto en una función auxiliar para coaccionar el tipo a `Value | undefined`, ya que algunas
versiones de TypeScript no infieren este tipo correctamente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](server.md#genericdocument) |
| `FieldPath` | extiende `string` |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:120](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L120)

***

### GenericTableInfo \{#generictableinfo\}

Ƭ **GenericTableInfo**: `Object`

Un tipo que describe el tipo de documento y los índices de una tabla.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `document` | [`GenericDocument`](server.md#genericdocument) |
| `fieldPaths` | [`GenericFieldPaths`](server.md#genericfieldpaths) |
| `indexes` | [`GenericTableIndexes`](server.md#generictableindexes) |
| `searchIndexes` | [`GenericTableSearchIndexes`](server.md#generictablesearchindexes) |
| `vectorIndexes` | [`GenericTableVectorIndexes`](server.md#generictablevectorindexes) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:145](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L145)

***

### DocumentByInfo \{#documentbyinfo\}

Ƭ **DocumentByInfo**&lt;`TableInfo`&gt;: `TableInfo`[`"document"`]

El tipo de un documento en una tabla para un [GenericTableInfo](server.md#generictableinfo) dado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:157](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L157)

***

### FieldPaths \{#fieldpaths\}

Ƭ **FieldPaths**&lt;`TableInfo`&gt;: `TableInfo`[`"fieldPaths"`]

Las rutas de campos de una tabla para un [GenericTableInfo](server.md#generictableinfo) determinado.

Estas pueden ser nombres de campos (como &quot;name&quot;) o referencias a campos de
objetos anidados (como &quot;properties.name&quot;).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L167)

***

### Indexes \{#indexes\}

Ƭ **Indexes**&lt;`TableInfo`&gt;: `TableInfo`[`"indexes"`]

Los índices de base de datos de una tabla para un [GenericTableInfo](server.md#generictableinfo) dado.

Será un objeto que asigna nombres de índices a los campos del índice.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:176](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L176)

***

### IndexNames \{#indexnames\}

Ƭ **IndexNames**&lt;`TableInfo`&gt;: keyof [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;

Los nombres de los índices de una tabla para un [GenericTableInfo](server.md#generictableinfo) determinado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:182](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L182)

***

### NamedIndex \{#namedindex\}

Ƭ **NamedIndex**&lt;`TableInfo`, `IndexName`&gt;: [`Indexes`](server.md#indexes)&lt;`TableInfo`&gt;[`IndexName`]

Extrae los campos de un índice de un [GenericTableInfo](server.md#generictableinfo) por nombre.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | extends [`IndexNames`](server.md#indexnames)&lt;`TableInfo`&gt; |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:189](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L189)

***

### SearchIndexes \{#searchindexes\}

Ƭ **SearchIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"searchIndexes"`]

Los índices de búsqueda de una tabla para un [GenericTableInfo](server.md#generictableinfo) determinado.

Será un objeto que asigna nombres de índices a la configuración del índice de búsqueda.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:200](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L200)

***

### SearchIndexNames \{#searchindexnames\}

Ƭ **SearchIndexNames**&lt;`TableInfo`&gt;: keyof [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;

Los nombres de los índices de búsqueda en una tabla para un [GenericTableInfo](server.md#generictableinfo) determinado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:207](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L207)

***

### NamedSearchIndex \{#namedsearchindex\}

Ƭ **NamedSearchIndex**&lt;`TableInfo`, `IndexName`&gt;: [`SearchIndexes`](server.md#searchindexes)&lt;`TableInfo`&gt;[`IndexName`]

Extrae la configuración de un índice de búsqueda de un [GenericTableInfo](server.md#generictableinfo) por su nombre.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | extiende [`SearchIndexNames`](server.md#searchindexnames)&lt;`TableInfo`&gt; |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:214](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L214)

***

### VectorIndexes \{#vectorindexes\}

Ƭ **VectorIndexes**&lt;`TableInfo`&gt;: `TableInfo`[`"vectorIndexes"`]

Los índices vectoriales de una tabla para un [GenericTableInfo](server.md#generictableinfo) dado.

Será un objeto que asigna nombres de índices a la configuración del índice vectorial.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:225](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L225)

***

### VectorIndexNames \{#vectorindexnames\}

Ƭ **VectorIndexNames**&lt;`TableInfo`&gt;: keyof [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;

Los nombres de los índices vectoriales de una tabla para un [GenericTableInfo](server.md#generictableinfo) dado.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extiende de [`GenericTableInfo`](server.md#generictableinfo) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:232](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L232)

***

### NamedVectorIndex \{#namedvectorindex\}

Ƭ **NamedVectorIndex**&lt;`TableInfo`, `IndexName`&gt;: [`VectorIndexes`](server.md#vectorindexes)&lt;`TableInfo`&gt;[`IndexName`]

Extrae la configuración de un índice vectorial de un [GenericTableInfo](server.md#generictableinfo) por nombre.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableInfo` | extends [`GenericTableInfo`](server.md#generictableinfo) |
| `IndexName` | extends [`VectorIndexNames`](server.md#vectorindexnames)&lt;`TableInfo`&gt; |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:239](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L239)

***

### GenericDataModel \{#genericdatamodel\}

Ƭ **GenericDataModel**: `Record`&lt;`string`, [`GenericTableInfo`](server.md#generictableinfo)&gt;

Un tipo que describe las tablas de un proyecto de Convex.

Está pensado para generarse mediante generación de código con `npx convex dev`.

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:252](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L252)

***

### AnyDataModel \{#anydatamodel\}

Ƭ **AnyDataModel**: `Object`

Un [GenericDataModel](server.md#genericdatamodel) que considera que los documentos son de tipo `any` y no
admite índices.

Este es el valor predeterminado antes de definir un esquema.

#### Firma de índice \{#index-signature\}

▪ [tableName: `string`]: &#123; `document`: `any` ; `fieldPaths`: [`GenericFieldPaths`](server.md#genericfieldpaths) ; `indexes`: {} ; `searchIndexes`: {} ; `vectorIndexes`: {}  &#125;

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:261](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L261)

***

### TableNamesInDataModel \{#tablenamesindatamodel\}

Ƭ **TableNamesInDataModel**&lt;`DataModel`&gt;: keyof `DataModel` &amp; `string`

Tipo que representa todos los nombres de tablas definidos en un [GenericDataModel](server.md#genericdatamodel).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:275](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L275)

***

### NamedTableInfo \{#namedtableinfo\}

Ƭ **NamedTableInfo**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`]

Extrae el `TableInfo` de una tabla de un [GenericDataModel](server.md#genericdatamodel) por su nombre.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extends keyof `DataModel` |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:284](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L284)

***

### DocumentByName \{#documentbyname\}

Ƭ **DocumentByName**&lt;`DataModel`, `TableName`&gt;: `DataModel`[`TableName`][`"document"`]

El tipo de un documento en un [GenericDataModel](server.md#genericdatamodel) según el nombre de la tabla.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extiende [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |

#### Definido en \{#defined-in\}

[server/data&#95;model.ts:293](https://github.com/get-convex/convex-js/blob/main/src/server/data_model.ts#L293)

***

### ExpressionOrValue \{#expressionorvalue\}

Ƭ **ExpressionOrValue**&lt;`T`&gt;: [`Expression`](../classes/server.Expression.md)&lt;`T`&gt; | `T`

Una [`Expression`](../classes/server.Expression.md) o un [Valor](values.md#value) constante

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Value`](values.md#value) | `undefined` |

#### Definido en \{#defined-in\}

[server/filter&#95;builder.ts:38](https://github.com/get-convex/convex-js/blob/main/src/server/filter_builder.ts#L38)

***

### Cursor \{#cursor\}

Ƭ **Cursor**: `string`

Un identificador opaco que se usa para paginar una consulta de base de datos.

Los cursores se devuelven por [paginate](../interfaces/server.OrderedQuery.md#paginate) y representan el
punto de la consulta donde terminó la página de resultados.

Para continuar paginando, pasa el cursor de vuelta a
[paginate](../interfaces/server.OrderedQuery.md#paginate) dentro del objeto [PaginationOptions](../interfaces/server.PaginationOptions.md) para
obtener otra página de resultados.

Nota: Los cursores solo se pueden pasar a *exactamente* la misma consulta de base de datos con la que
se generaron. No puedes reutilizar un cursor entre diferentes
consultas de base de datos.

#### Definido en \{#defined-in\}

[server/pagination.ts:21](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L21)

***

### GenericMutationCtxWithTable \{#genericmutationctxwithtable\}

Ƭ **GenericMutationCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseWriterWithTable`](../interfaces/server.GenericDatabaseWriterWithTable.md)&lt;`DataModel`&gt;  &#125;

Un conjunto de servicios para utilizar dentro de funciones de mutación de Convex.

El contexto de mutación se pasa como primer argumento a cualquier función de
mutación de Convex que se ejecute en el servidor.

Si estás usando generación de código, usa el tipo `MutationCtx` en
`convex/_generated/server.d.ts`, que está tipado para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende de [`GenericDataModel`](server.md#genericdatamodel) |

#### Definido en \{#defined-in\}

[server/registration.ts:109](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L109)

***

### GenericQueryCtxWithTable \{#genericqueryctxwithtable\}

Ƭ **GenericQueryCtxWithTable**&lt;`DataModel`&gt;: `Omit`&lt;[`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, `"db"`&gt; &amp; &#123; `db`: [`GenericDatabaseReaderWithTable`](../interfaces/server.GenericDatabaseReaderWithTable.md)&lt;`DataModel`&gt;  &#125;

Un conjunto de servicios para utilizar dentro de las funciones de consulta de Convex.

El contexto de consulta se pasa como primer argumento a cualquier función
de consulta de Convex que se ejecute en el servidor.

Esto se diferencia de `MutationCtx` porque todos los servicios son de solo lectura.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extends [`GenericDataModel`](server.md#genericdatamodel) |

#### Definido en \{#defined-in\}

[server/registration.ts:167](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L167)

***

### DefaultFunctionArgs \{#defaultfunctionargs\}

Ƭ **DefaultFunctionArgs**: `Record`&lt;`string`, `unknown`&gt;

El tipo de argumentos predeterminado para una función de consulta, mutación o acción de Convex.

Las funciones de Convex siempre reciben un objeto de argumentos que asigna cada nombre de argumento a su valor.

#### Definido en \{#defined-in\}

[server/registration.ts:278](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L278)

***

### ArgsArray \{#argsarray\}

Ƭ **ArgsArray**: `OneArgArray` | `NoArgsArray`

Un array de argumentos para una función de Convex.

Las funciones de Convex pueden aceptar ya sea un único objeto [DefaultFunctionArgs](server.md#defaultfunctionargs) o ningún
argumento en absoluto.

#### Definido en \{#defined-in\}

[server/registration.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L301)

***

### ArgsArrayToObject \{#argsarraytoobject\}

Ƭ **ArgsArrayToObject**&lt;`Args`&gt;: `Args` extends `OneArgArray`&lt;infer ArgsObject&gt; ? `ArgsObject` : `EmptyObject`

Convierte un [ArgsArray](server.md#argsarray) en un único tipo de objeto.

Los arrays de argumentos vacíos se convierten en EmptyObject.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Args` | extiende [`ArgsArray`](server.md#argsarray) |

#### Definido en \{#defined-in\}

[server/registration.ts:316](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L316)

***

### FunctionVisibility \{#functionvisibility\}

Ƭ **FunctionVisibility**: `"public"` | `"internal"`

Tipo que representa la visibilidad de una función de Convex.

#### Definido en \{#defined-in\}

[server/registration.ts:324](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L324)

***

### RegisteredMutation \{#registeredmutation\}

Ƭ **RegisteredMutation**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isMutation`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

Una función de mutación que forma parte de esta aplicación.

Puedes crear una mutación envolviendo tu función con
[mutationGeneric](server.md#mutationgeneric) o [internalMutationGeneric](server.md#internalmutationgeneric) y exportándola.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### Definido en \{#defined-in\}

[server/registration.ts:347](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L347)

***

### RegisteredQuery \{#registeredquery\}

Ƭ **RegisteredQuery**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isQuery`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

Una función de consulta que forma parte de esta aplicación.

Puedes crear una consulta envolviendo tu función con
[queryGeneric](server.md#querygeneric) o [internalQueryGeneric](server.md#internalquerygeneric) y exportándola.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | extiende [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### Definido en \{#defined-in\}

[server/registration.ts:376](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L376)

***

### RegisteredAction \{#registeredaction\}

Ƭ **RegisteredAction**&lt;`Visibility`, `Args`, `Returns`&gt;: &#123; `isConvexFunction`: `true` ; `isAction`: `true`  &#125; &amp; `VisibilityProperties`&lt;`Visibility`&gt;

Una acción que forma parte de esta app.

Puedes crear una acción envolviendo tu función con
[actionGeneric](server.md#actiongeneric) o [internalActionGeneric](server.md#internalactiongeneric) y exportándola.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Visibility` | extends [`FunctionVisibility`](server.md#functionvisibility) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `Returns` | `Returns` |

#### Definido en \{#defined-in\}

[server/registration.ts:405](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L405)

***

### PublicHttpAction \{#publichttpaction\}

Ƭ **PublicHttpAction**: `Object`

Una acción HTTP que pertenece a la API pública de esta aplicación.

Puedes crear acciones HTTP públicas envolviendo tu función en
[httpActionGeneric](server.md#httpactiongeneric) y exportarla.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `isHttp` | `true` |

#### Definido en \{#defined-in\}

[server/registration.ts:434](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L434)

***

### UnvalidatedFunction \{#unvalidatedfunction\}

Ƭ **UnvalidatedFunction**&lt;`Ctx`, `Args`, `Returns`&gt;: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns` | &#123; `handler`: (`ctx`: `Ctx`, ...`args`: `Args`) =&gt; `Returns`  &#125;

**`Deprecated`**

-- Consulta la definición de tipo de `MutationBuilder` o similares para
los tipos usados al definir funciones de Convex.

Definición de una consulta, mutación o acción de Convex sin
validación de argumentos.

Las funciones de Convex siempre reciben un objeto de contexto como su primer argumento
y un objeto de argumentos (opcional) como su segundo argumento.

Esto se puede escribir como una función así:

```js
import { query } from "./_generated/server";

export const func = query(({ db }, { arg }) => {...});
```

o como un objeto, por ejemplo:

```js
import { query } from "./_generated/server";

export const func = query({
  handler: ({ db }, { arg }) => {...},
});
```

Consulta [ValidatedFunction](../interfaces/server.ValidatedFunction.md) para añadir validación de argumentos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Ctx` | `Ctx` |
| `Args` | extiende de [`ArgsArray`](server.md#argsarray) |
| `Returns` | `Returns` |

#### Definido en \{#defined-in\}

[server/registration.ts:472](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L472)

***

### ReturnValueForOptionalValidator \{#returnvalueforoptionalvalidator\}

Ƭ **ReturnValueForOptionalValidator**&lt;`ReturnsValidator`&gt;: [`ReturnsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `ValidatorTypeToReturnType`&lt;[`Infer`](values.md#infer)&lt;`ReturnsValidator`&gt;&gt; : [`ReturnsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `ValidatorTypeToReturnType`&lt;[`ObjectType`](values.md#objecttype)&lt;`ReturnsValidator`&gt;&gt; : `any`

Existen varias sintaxis para definir una función en Convex:

```
 - query(async (ctx, args) => {...})
 - query({ handler: async (ctx, args) => {...} })
 - query({ args: { a: v.string }, handler: async (ctx, args) => {...} } })
 - query({ args: { a: v.string }, returns: v.string(), handler: async (ctx, args) => {...} } })
```

En cada uno de estos casos, queremos inferir correctamente el tipo de los argumentos y
del valor de retorno, dando preferencia al tipo derivado de un validador si se proporciona.

Para evitar tener una sobrecarga separada para cada uno, lo que aparecería en los mensajes de error,
usamos los parámetros de tipo -- ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs.

El tipo de ReturnValue y OneOrZeroArgs está restringido por el tipo de ArgsValidator y
ReturnsValidator si están presentes, y se infiere a partir de cualquier anotación de tipo explícita en los
argumentos o en el valor de retorno de la función.

A continuación se muestran algunos tipos de utilidad para obtener las restricciones de tipo apropiadas basadas en
un validador opcional.

Trucos adicionales:

* Usamos Validator | void en lugar de Validator | undefined porque este último no
  funciona con `strictNullChecks`, ya que es equivalente simplemente a `Validator`.
* Usamos un tipo de tupla de longitud 1 para evitar la distribución sobre la unión
  https://github.com/microsoft/TypeScript/issues/29368#issuecomment-453529532

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ReturnsValidator` | extends [`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### Definido en \{#defined-in\}

[server/registration.ts:574](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L574)

***

### ArgsArrayForOptionalValidator \{#argsarrayforoptionalvalidator\}

Ƭ **ArgsArrayForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? `OneArgArray`&lt;[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;&gt; : [`ArgsArray`](server.md#argsarray)

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### Definido en \{#defined-in\}

[server/registration.ts:582](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L582)

***

### DefaultArgsForOptionalValidator \{#defaultargsforoptionalvalidator\}

Ƭ **DefaultArgsForOptionalValidator**&lt;`ArgsValidator`&gt;: [`ArgsValidator`] extends [[`Validator`](values.md#validator)&lt;`any`, `any`, `any`&gt;] ? [[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;] : [`ArgsValidator`] extends [[`PropertyValidators`](values.md#propertyvalidators)] ? [[`ObjectType`](values.md#objecttype)&lt;`ArgsValidator`&gt;] : `OneArgArray`

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends [`GenericValidator`](values.md#genericvalidator) | [`PropertyValidators`](values.md#propertyvalidators) | `void` |

#### Definido en \{#defined-in\}

[server/registration.ts:590](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L590)

***

### MutationBuilder \{#mutationbuilder\}

Ƭ **MutationBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Tipo auxiliar interno utilizado por la generación de código de Convex.

Se utiliza para darle a [mutationGeneric](server.md#mutationgeneric) un tipo específico de tu modelo de datos.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### Devuelve \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:604](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L604)

***

### MutationBuilderWithTable \{#mutationbuilderwithtable\}

Ƭ **MutationBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`mutation`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Tipo auxiliar interno utilizado por la generación de código de Convex.

Se utiliza para proporcionar a [mutationGeneric](server.md#mutationgeneric) un tipo específico para tu modelo de datos.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtxWithTable`](server.md#genericmutationctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### Devuelve \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:697](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L697)

***

### QueryBuilder \{#querybuilder\}

Ƭ **QueryBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Utilidad de tipo interna utilizada por la generación de código de Convex.

Se utiliza para darle a [queryGeneric](server.md#querygeneric) un tipo específico para tu modelo de datos.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### Devuelve \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:790](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L790)

***

### QueryBuilderWithTable \{#querybuilderwithtable\}

Ƭ **QueryBuilderWithTable**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`query`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Tipo auxiliar interno utilizado por la generación de código de Convex.

Se utiliza para darle a [queryGeneric](server.md#querygeneric) un tipo específico según tu modelo de datos.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtxWithTable`](server.md#genericqueryctxwithtable)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### Devuelve \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:879](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L879)

***

### ActionBuilder \{#actionbuilder\}

Ƭ **ActionBuilder**&lt;`DataModel`, `Visibility`&gt;: &lt;ArgsValidator, ReturnsValidator, ReturnValue, OneOrZeroArgs&gt;(`func`: &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`) =&gt; [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `Visibility` | extiende [`FunctionVisibility`](server.md#functionvisibility) |

#### Declaración de tipo \{#type-declaration\}

▸ &lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Tipo auxiliar interno utilizado por la generación de código de Convex.

Se utiliza para asignar a [actionGeneric](server.md#actiongeneric) un tipo específico de tu modelo de datos.

##### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`DataModel`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

##### Devuelve \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`Visibility`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

#### Definido en \{#defined-in\}

[server/registration.ts:968](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L968)

***

### HttpActionBuilder \{#httpactionbuilder\}

Ƭ **HttpActionBuilder**: (`func`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt;) =&gt; [`PublicHttpAction`](server.md#publichttpaction)

#### Declaración de tipo \{#type-declaration\}

▸ (`func`): [`PublicHttpAction`](server.md#publichttpaction)

Tipo auxiliar interno utilizado por la generación de código de Convex.

Se utiliza para asignar a [httpActionGeneric](server.md#httpactiongeneric) un tipo específico para tu modelo de datos
y tus funciones.

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; |

##### Devuelve \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

#### Definido en \{#defined-in\}

[server/registration.ts:1063](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L1063)

***

### RoutableMethod \{#routablemethod\}

Ƭ **RoutableMethod**: typeof [`ROUTABLE_HTTP_METHODS`](server.md#routable_http_methods)[`number`]

Un tipo que representa los métodos admitidos por las acciones HTTP de Convex.

HEAD lo gestiona Convex ejecutando GET y eliminando el cuerpo.
CONNECT no está soportado y no lo estará.
TRACE no está soportado y no lo estará.

#### Definido en \{#defined-in\}

[server/router.ts:31](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L31)

***

### RouteSpecWithPath \{#routespecwithpath\}

Ƭ **RouteSpecWithPath**: `Object`

Un tipo que representa una ruta a una acción HTTP utilizando una coincidencia exacta del path de la URL de la solicitud.

Se usa en [HttpRouter](../classes/server.HttpRouter.md) para enrutar solicitudes a acciones HTTP.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `path` | `string` | Ruta exacta de la solicitud HTTP que se debe enrutar. |
| `method` | [`RoutableMethod`](server.md#routablemethod) | Método HTTP (&quot;GET&quot;, &quot;POST&quot;, ...) que se debe enrutar. |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | La acción HTTP que se va a ejecutar. |

#### Definido en \{#defined-in\}

[server/router.ts:56](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L56)

***

### RouteSpecWithPathPrefix \{#routespecwithpathprefix\}

Ƭ **RouteSpecWithPathPrefix**: `Object`

Un tipo que representa una ruta para una acción HTTP que usa una coincidencia de prefijo en la ruta de la URL de la solicitud.

Utilizado por [HttpRouter](../classes/server.HttpRouter.md) para enrutar solicitudes a acciones HTTP.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `pathPrefix` | `string` | Prefijo de ruta de la solicitud HTTP para enrutar. Las solicitudes cuya ruta comience con este valor se enrutarán a la acción HTTP. |
| `method` | [`RoutableMethod`](server.md#routablemethod) | Método HTTP («GET», «POST», …) que se va a enrutar. |
| `handler` | [`PublicHttpAction`](server.md#publichttpaction) | La acción HTTP que se debe ejecutar. |

#### Definido en \{#defined-in\}

[server/router.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L78)

***

### RouteSpec \{#routespec\}

Ƭ **RouteSpec**: [`RouteSpecWithPath`](server.md#routespecwithpath) | [`RouteSpecWithPathPrefix`](server.md#routespecwithpathprefix)

Tipo que representa una ruta a una acción HTTP.

Utilizado por [HttpRouter](../classes/server.HttpRouter.md) para enrutar solicitudes a acciones HTTP.

#### Definido en \{#defined-in\}

[server/router.ts:101](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L101)

***

### SchedulableFunctionReference \{#schedulablefunctionreference\}

Ƭ **SchedulableFunctionReference**: [`FunctionReference`](server.md#functionreference)&lt;`"mutation"` | `"action"`, `"public"` | `"internal"`&gt;

Un [FunctionReference](server.md#functionreference) que puede programarse para ejecutarse en el futuro.

Las funciones programables son mutaciones y acciones públicas o internas.

#### Definido en \{#defined-in\}

[server/scheduler.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/scheduler.ts#L11)

***

### GenericSchema \{#genericschema\}

Ƭ **GenericSchema**: `Record`&lt;`string`, [`TableDefinition`](../classes/server.TableDefinition.md)&gt;

Un tipo que describe el esquema de un proyecto de Convex.

Este tipo debe construirse usando [defineSchema](server.md#defineschema), [defineTable](server.md#definetable)
y [v](values.md#v).

#### Definido en \{#defined-in\}

[server/schema.ts:645](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L645)

***

### DataModelFromSchemaDefinition \{#datamodelfromschemadefinition\}

Ƭ **DataModelFromSchemaDefinition**&lt;`SchemaDef`&gt;: `MaybeMakeLooseDataModel`&lt;&#123; [TableName in keyof SchemaDef[&quot;tables&quot;] &amp; string]: SchemaDef[&quot;tables&quot;][TableName] extends TableDefinition&lt;infer DocumentType, infer Indexes, infer SearchIndexes, infer VectorIndexes&gt; ? Object : never &#125;, `SchemaDef`[`"strictTableNameTypes"`]&gt;

Tipo interno utilizado en la generación de código de Convex.

Convierte un [SchemaDefinition](../classes/server.SchemaDefinition.md) en un [GenericDataModel](server.md#genericdatamodel).

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `SchemaDef` | extends [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`any`, `boolean`&gt; |

#### Definido en \{#defined-in\}

[server/schema.ts:786](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L786)

***

### SystemTableNames \{#systemtablenames\}

Ƭ **SystemTableNames**: [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;[`SystemDataModel`](../interfaces/server.SystemDataModel.md)&gt;

#### Definido en \{#defined-in\}

[server/schema.ts:844](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L844)

***

### StorageId \{#storageid\}

Ƭ **StorageId**: `string`

Una referencia a un archivo en el almacenamiento.

Se utiliza en [StorageReader](../interfaces/server.StorageReader.md) y [StorageWriter](../interfaces/server.StorageWriter.md), que son accesibles en
consultas y mutaciones de Convex mediante QueryCtx y MutationCtx, respectivamente.

#### Definido en \{#defined-in\}

[server/storage.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L11)

***

### FileStorageId \{#filestorageid\}

Ƭ **FileStorageId**: [`GenericId`](values.md#genericid)&lt;`"_storage"`&gt; | [`StorageId`](server.md#storageid)

#### Definido en \{#defined-in\}

[server/storage.ts:12](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L12)

***

### FileMetadata \{#filemetadata\}

Ƭ **FileMetadata**: `Object`

Metadatos de un solo archivo, tal como los devuelve [storage.getMetadata](../interfaces/server.StorageReader.md#getmetadata).

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `storageId` | [`StorageId`](server.md#storageid) | ID para hacer referencia al archivo (p. ej., mediante [storage.getUrl](../interfaces/server.StorageReader.md#geturl)) |
| `sha256` | `string` | Suma de comprobación sha256 del contenido del archivo codificada en hexadecimal |
| `size` | `number` | Tamaño del archivo en bytes |
| `contentType` | `string` | `null` | Tipo de contenido (Content-Type) del archivo si se proporcionó al subirlo |

#### Definido en \{#defined-in\}

[server/storage.ts:18](https://github.com/get-convex/convex-js/blob/main/src/server/storage.ts#L18)

***

### SystemFields \{#systemfields\}

Ƭ **SystemFields**: `Object`

Los campos que Convex agrega automáticamente a los documentos, sin incluir `_id`.

Este es un tipo de objeto que asigna nombres de campos a tipos de campos.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `_creationTime` | `number` |

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:11](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L11)

***

### IdField \{#idfield\}

Ƭ **IdField**&lt;`TableName`&gt;: `Object`

El campo `_id` que Convex añade automáticamente a los documentos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `TableName` | extiende `string` |

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `_id` | [`GenericId`](values.md#genericid)&lt;`TableName`&gt; |

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:19](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L19)

***

### WithoutSystemFields \{#withoutsystemfields\}

Ƭ **WithoutSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`BetterOmit`](server.md#betteromit)&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;

Un documento de Convex en el que se han omitido los campos de sistema como `_id` y `_creationTime`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extiende [`GenericDocument`](server.md#genericdocument) |

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:28](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L28)

***

### WithOptionalSystemFields \{#withoptionalsystemfields\}

Ƭ **WithOptionalSystemFields**&lt;`Document`&gt;: [`Expand`](server.md#expand)&lt;[`WithoutSystemFields`](server.md#withoutsystemfields)&lt;`Document`&gt; &amp; `Partial`&lt;`Pick`&lt;`Document`, keyof [`SystemFields`](server.md#systemfields) | `"_id"`&gt;&gt;&gt;

Documento de Convex en el que campos de sistema como `_id` y `_creationTime` son opcionales.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Document` | extends [`GenericDocument`](server.md#genericdocument) |

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:37](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L37)

***

### SystemIndexes \{#systemindexes\}

Ƭ **SystemIndexes**: `Object`

Los índices que Convex añade automáticamente a cada tabla.

Es un objeto que asigna nombres de índices a rutas de campos de los índices.

#### Declaración de tipo \{#type-declaration\}

| Nombre | Tipo |
| :------ | :------ |
| `by_id` | [`"_id"`] |
| `by_creation_time` | [`"_creationTime"`] |

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:48](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L48)

***

### IndexTiebreakerField \{#indextiebreakerfield\}

Ƭ **IndexTiebreakerField**: `"_creationTime"`

Convex agrega automáticamente &quot;&#95;creationTime&quot; al final de cada índice para romper empates cuando todos los demás campos son idénticos.

#### Definido en \{#defined-in\}

[server/system&#95;fields.ts:61](https://github.com/get-convex/convex-js/blob/main/src/server/system_fields.ts#L61)

***

### VectorSearch \{#vectorsearch\}

Ƭ **VectorSearch**&lt;`DataModel`, `TableName`, `IndexName`&gt;: (`tableName`: `TableName`, `indexName`: `IndexName`, `query`: [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt;) =&gt; `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DataModel` | extiende [`GenericDataModel`](server.md#genericdatamodel) |
| `TableName` | extiende [`TableNamesInDataModel`](server.md#tablenamesindatamodel)&lt;`DataModel`&gt; |
| `IndexName` | extiende [`VectorIndexNames`](server.md#vectorindexnames)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;&gt; |

#### Declaración de tipo \{#type-declaration\}

▸ (`tableName`, `indexName`, `query`): `Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

##### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `tableName` | `TableName` |
| `indexName` | `IndexName` |
| `query` | [`VectorSearchQuery`](../interfaces/server.VectorSearchQuery.md)&lt;[`NamedTableInfo`](server.md#namedtableinfo)&lt;`DataModel`, `TableName`&gt;, `IndexName`&gt; |

##### Devuelve \{#returns\}

`Promise`&lt;&#123; `_id`: [`GenericId`](values.md#genericid)&lt;`TableName`&gt; ; `_score`: `number`  &#125;[]&gt;

#### Definido en \{#defined-in\}

[server/vector&#95;search.ts:55](https://github.com/get-convex/convex-js/blob/main/src/server/vector_search.ts#L55)

***

### Expand \{#expand\}

Ƭ **Expand**&lt;`ObjectType`&gt;: `ObjectType` extends `Record`&lt;`any`, `any`&gt; ? &#123; [Key in keyof ObjectType]: ObjectType[Key] &#125; : `never`

¡Truco! Este tipo hace que TypeScript simplifique la forma en que muestra los tipos de objeto.

Funcionalmente es la identidad para los tipos de objeto, pero en la práctica puede
simplificar expresiones como `A & B`.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ObjectType` | extends `Record`&lt;`any`, `any`&gt; |

#### Definido en \{#defined-in\}

[type&#95;utils.ts:12](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L12)

***

### BetterOmit \{#betteromit\}

Ƭ **BetterOmit**&lt;`T`, `K`&gt;: &#123; [Property in keyof T as Property extends K ? never : Property]: T[Property] &#125;

Un tipo `Omit<>` que:

1. Se aplica a cada miembro de una unión.
2. Conserva la firma de índice del tipo subyacente.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | `T` |
| `K` | extends keyof `T` |

#### Definido en \{#defined-in\}

[type&#95;utils.ts:24](https://github.com/get-convex/convex-js/blob/main/src/type_utils.ts#L24)

## Variables \{#variables\}

### anyApi \{#anyapi\}

• `Const` **anyApi**: [`AnyApi`](server.md#anyapi)

Una utilidad para construir [FunctionReference](server.md#functionreference)s en proyectos que
no usan generación de código.

Puedes crear una referencia a una función como:

```js
const reference = anyApi.myModule.myFunction;
```

Esto permite acceder a cualquier ruta independientemente de qué directorios y módulos
haya en tu proyecto. Todas las referencias de funciones tienen el tipo
AnyFunctionReference.

Si estás usando generación de código, usa `api` de `convex/_generated/api`
en su lugar. Será más seguro a nivel de tipos y producirá un mejor autocompletado
en tu editor.

#### Definido en \{#defined-in\}

[server/api.ts:427](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L427)

***

### paginationOptsValidator \{#paginationoptsvalidator\}

• `Const` **paginationOptsValidator**: [`VObject`](../classes/values.VObject.md)&lt;&#123; `id`: `undefined` | `number` ; `endCursor`: `undefined` | `null` | `string` ; `maximumRowsRead`: `undefined` | `number` ; `maximumBytesRead`: `undefined` | `number` ; `numItems`: `number` ; `cursor`: `null` | `string`  &#125;, &#123; `numItems`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`number`, `"required"`&gt; ; `cursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"required"`, `never`&gt; ; `endCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `id`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumRowsRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt; ; `maximumBytesRead`: [`VFloat64`](../classes/values.VFloat64.md)&lt;`undefined` | `number`, `"optional"`&gt;  &#125;, `"required"`, `"id"` | `"numItems"` | `"cursor"` | `"endCursor"` | `"maximumRowsRead"` | `"maximumBytesRead"`&gt;

Un [Validator](values.md#validator) para [PaginationOptions](../interfaces/server.PaginationOptions.md).

Incluye las propiedades estándar de [PaginationOptions](../interfaces/server.PaginationOptions.md) junto con
una propiedad opcional `id` para evitar el uso de caché, utilizada por [usePaginatedQuery](react.md#usepaginatedquery).

#### Definido en \{#defined-in\}

[server/pagination.ts:133](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L133)

***

### ROUTABLE_HTTP_METHODS \{#routable_http_methods\}

• `Const` **ROUTABLE&#95;HTTP&#95;METHODS**: readonly [`"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"OPTIONS"`, `"PATCH"`]

Lista de los métodos admitidos por las acciones HTTP de Convex.

HEAD es gestionado por Convex ejecutando GET y omitiendo el cuerpo.
CONNECT no es compatible ni lo será.
TRACE no es compatible ni lo será.

#### Definido en \{#defined-in\}

[server/router.ts:14](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L14)

## Funciones \{#functions\}

### getFunctionName \{#getfunctionname\}

▸ **getFunctionName**(`functionReference`): `string`

Obtiene el nombre de una función a partir de una [FunctionReference](server.md#functionreference).

El nombre es una cadena como &quot;myDir/myModule:myFunction&quot;. Si el nombre
exportado de la función es `"default"`, se omite el nombre de la función
(p. ej., &quot;myDir/myModule&quot;).

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `functionReference` | `AnyFunctionReference` | Un [FunctionReference](server.md#functionreference) del que obtener su nombre. |

#### Devuelve \{#returns\}

`string`

Cadena con el nombre de la función.

#### Definido en \{#defined-in\}

[server/api.ts:78](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L78)

***

### makeFunctionReference \{#makefunctionreference\}

▸ **makeFunctionReference**&lt;`type`, `args`, `ret`&gt;(`name`): [`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

Las instancias de `FunctionReference` generalmente provienen de código generado, pero en clientes personalizados
puede ser útil poder construir una manualmente.

Las referencias de funciones reales son objetos vacíos en tiempo de ejecución, pero la misma interfaz
se puede implementar con un objeto para pruebas y para clientes que no usan
generación de código.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `type` | extends [`FunctionType`](server.md#functiontype) |
| `args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) = `any` |
| `ret` | `any` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `name` | `string` | El identificador de la función. Por ejemplo: `path/to/file:functionName` |

#### Devuelve \{#returns\}

[`FunctionReference`](server.md#functionreference)&lt;`type`, `"public"`, `args`, `ret`&gt;

#### Definido en \{#defined-in\}

[server/api.ts:122](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L122)

***

### filterApi \{#filterapi\}

▸ **filterApi**&lt;`API`, `Predicate`&gt;(`api`): [`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

Dado un `api` de tipo `API` y un subtipo de `FunctionReference`, devuelve un objeto de API
que contiene solo las referencias de función que coinciden.

```ts
const q = filterApi<typeof api, FunctionReference<"query">>(api)
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre |
| :------ |
| `API` |
| `Predicate` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `api` | `API` |

#### Devuelve \{#returns\}

[`FilterApi`](server.md#filterapi)&lt;`API`, `Predicate`&gt;

#### Definido en \{#defined-in\}

[server/api.ts:301](https://github.com/get-convex/convex-js/blob/main/src/server/api.ts#L301)

***

### createFunctionHandle \{#createfunctionhandle\}

▸ **createFunctionHandle**&lt;`Type`, `Args`, `ReturnType`&gt;(`functionReference`): `Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

Crea una referencia serializable a una función de Convex.
Pasar esta referencia a otro componente permite que ese componente llame a esta
función durante la ejecución de la función actual o en cualquier momento posterior.
Los function handles se usan igual que las FunctionReferences `api.folder.function`,
por ejemplo, `ctx.scheduler.runAfter(0, functionReference, args)`.

Una referencia de función es estable entre publicaciones de código, pero es posible
que la función de Convex a la que se refiere ya no exista.

Esta es una funcionalidad de los componentes, que están en beta.
Esta API es inestable y puede cambiar en versiones posteriores.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Type` | extends [`FunctionType`](server.md#functiontype) |
| `Args` | extends [`DefaultFunctionArgs`](server.md#defaultfunctionargs) |
| `ReturnType` | `ReturnType` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `functionReference` | [`FunctionReference`](server.md#functionreference)&lt;`Type`, `"public"` | `"internal"`, `Args`, `ReturnType`&gt; |

#### Devuelve \{#returns\}

`Promise`&lt;[`FunctionHandle`](server.md#functionhandle)&lt;`Type`, `Args`, `ReturnType`&gt;&gt;

#### Definido en \{#defined-in\}

[server/components/index.ts:54](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L54)

***

### defineComponent \{#definecomponent\}

▸ **defineComponent**&lt;`Exports`&gt;(`name`): [`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

Define un componente, una parte de un despliegue de Convex con recursos en un espacio de nombres.

El valor predeterminado,
la exportación predeterminada de un módulo como &quot;cool-component/convex.config.js&quot;
es un `@link ComponentDefinition&#125;, pero durante la evaluación de la definición del componente
este es su tipo en su lugar.

@param name El nombre debe ser alfanumérico y puede incluir guiones bajos. Normalmente se usan
minúsculas con guiones bajos como `"onboarding_flow_tracker"`.

Esta es una funcionalidad de los componentes, que están en beta.
Esta API es inestable y puede cambiar en versiones posteriores.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Exports` | extends `ComponentExports` = `any` |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `name` | `string` |

#### Devuelve \{#returns\}

[`ComponentDefinition`](server.md#componentdefinition)&lt;`Exports`&gt;

#### Definido en \{#defined-in\}

[server/components/index.ts:371](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L371)

***

### defineApp \{#defineapp\}

▸ **defineApp**(): `AppDefinition`

Adjunta componentes, piezas reutilizables de un despliegue de Convex, a esta aplicación de Convex.

Esta es una funcionalidad de los componentes, que están en beta.
Esta API es inestable y puede cambiar en versiones futuras.

#### Devuelve \{#returns\}

`AppDefinition`

#### Definido en \{#defined-in\}

[server/components/index.ts:397](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L397)

***

### componentsGeneric \{#componentsgeneric\}

▸ **componentsGeneric**(): [`AnyChildComponents`](server.md#anychildcomponents)

#### Devuelve \{#returns\}

[`AnyChildComponents`](server.md#anychildcomponents)

#### Definido en \{#defined-in\}

[server/components/index.ts:452](https://github.com/get-convex/convex-js/blob/main/src/server/components/index.ts#L452)

***

### getFunctionAddress \{#getfunctionaddress\}

▸ **getFunctionAddress**(`functionReference`): &#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `functionReference` | `any` |

#### Devuelve \{#returns\}

&#123; `functionHandle`: `string` = functionReference; `name?`: `undefined` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name`: `any` ; `reference?`: `undefined` = referencePath &#125; | &#123; `functionHandle?`: `undefined` = functionReference; `name?`: `undefined` ; `reference`: `string` = referencePath &#125;

#### Definido en \{#defined-in\}

[server/components/paths.ts:20](https://github.com/get-convex/convex-js/blob/main/src/server/components/paths.ts#L20)

***

### cronJobs \{#cronjobs\}

▸ **cronJobs**(): [`Crons`](../classes/server.Crons.md)

Crea un objeto CronJobs para programar tareas periódicas.

```js
// convex/crons.js
import { cronJobs } from 'convex/server';
import { api } from "./_generated/api";

const crons = cronJobs();
crons.weekly(
  "weekly re-engagement email",
  {
    hourUTC: 17, // (9:30 a. m. hora del Pacífico/10:30 a. m. hora de verano del Pacífico)
    minuteUTC: 30,
  },
  api.emails.send
)
export default crons;
```

#### Devuelve \{#returns\}

[`Crons`](../classes/server.Crons.md)

#### Definido en \{#defined-in\}

[server/cron.ts:180](https://github.com/get-convex/convex-js/blob/main/src/server/cron.ts#L180)

***

### mutationGeneric \{#mutationgeneric\}

▸ **mutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una mutación en la API pública de esta aplicación de Convex.

Esta función podrá modificar tu base de datos de Convex y será accesible desde el cliente.

Si estás usando generación de código, usa la función `mutation` en
`convex/_generated/server.d.ts`, que está tipada según tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### Devuelve \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La mutación envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### internalMutationGeneric \{#internalmutationgeneric\}

▸ **internalMutationGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`mutation`): [`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una mutación que solo es accesible desde otras funciones de Convex (pero no desde el cliente).

Esta función podrá modificar tu base de datos de Convex. No será accesible desde el cliente.

Si estás usando generación de código, usa la función `internalMutation` en
`convex/_generated/server.d.ts`, que tiene tipos específicos para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `mutation` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericMutationCtx`](../interfaces/server.GenericMutationCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### Devuelve \{#returns\}

[`RegisteredMutation`](server.md#registeredmutation)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La mutación envuelta. Inclúyela como un `export` para darle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:608](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L608)

***

### queryGeneric \{#querygeneric\}

▸ **queryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una consulta en la API pública de esta aplicación de Convex.

Esta función tendrá permisos para leer tu base de datos de Convex y será accesible desde el cliente.

Si estás usando generación de código, usa la función `query` en
`convex/_generated/server.d.ts`, que está tipada para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### Devuelve \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La consulta envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### internalQueryGeneric \{#internalquerygeneric\}

▸ **internalQueryGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`query`): [`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una consulta a la que solo se puede acceder desde otras funciones de Convex (no desde el cliente).

Esta función podrá leer datos de tu base de datos de Convex. No será accesible desde el cliente.

Si estás usando generación de código, usa la función `internalQuery` en
`convex/_generated/server.d.ts`, que está tipada según tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `query` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericQueryCtx`](../interfaces/server.GenericQueryCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` |

#### Devuelve \{#returns\}

[`RegisteredQuery`](server.md#registeredquery)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La consulta envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:794](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L794)

***

### actionGeneric \{#actiongeneric\}

▸ **actionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una acción en la API pública de esta aplicación de Convex.

Si utilizas generación de código, usa la función `action` en
`convex/_generated/server.d.ts`, que tiene tipado específico para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | La función. Recibe un [GenericActionCtx](../interfaces/server.GenericActionCtx.md) como primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"public"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La función envuelta. Inclúyela como un `export` para darle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### internalActionGeneric \{#internalactiongeneric\}

▸ **internalActionGeneric**&lt;`ArgsValidator`, `ReturnsValidator`, `ReturnValue`, `OneOrZeroArgs`&gt;(`func`): [`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

Define una acción accesible únicamente desde otras funciones de Convex (pero no desde el cliente).

Si estás usando generación de código, usa la función `internalAction` en
`convex/_generated/server.d.ts`, que tiene tipos definidos para tu modelo de datos.

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `ArgsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnsValidator` | extends `void` | [`Validator`](values.md#validator)&lt;`any`, `"required"`, `any`&gt; | [`PropertyValidators`](values.md#propertyvalidators) |
| `ReturnValue` | extends `any` = `any` |
| `OneOrZeroArgs` | extends [`ArgsArray`](server.md#argsarray) | `OneArgArray`&lt;[`Infer`](values.md#infer)&lt;`ArgsValidator`&gt;&gt; | `OneArgArray`&lt;[`Expand`](server.md#expand)&lt;&#123; [Property in string | number | symbol]?: Exclude&lt;Infer&lt;ArgsValidator[Property]&gt;, undefined&gt; &#125; &amp; &#123; [Property in string | number | symbol]: Infer&lt;ArgsValidator[Property]&gt; &#125;&gt;&gt; = [`DefaultArgsForOptionalValidator`](server.md#defaultargsforoptionalvalidator)&lt;`ArgsValidator`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `func` | &#123; `args?`: `ArgsValidator` ; `returns?`: `ReturnsValidator` ; `handler`: (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue`  &#125; | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;`any`&gt;, ...`args`: `OneOrZeroArgs`) =&gt; `ReturnValue` | La función. Recibe un [GenericActionCtx](../interfaces/server.GenericActionCtx.md) como primer argumento. |

#### Devuelve \{#returns\}

[`RegisteredAction`](server.md#registeredaction)&lt;`"internal"`, [`ArgsArrayToObject`](server.md#argsarraytoobject)&lt;`OneOrZeroArgs`&gt;, `ReturnValue`&gt;

La función envuelta. Inclúyela como un `export` para asignarle un nombre y hacerla accesible.

#### Definido en \{#defined-in\}

[server/registration.ts:972](https://github.com/get-convex/convex-js/blob/main/src/server/registration.ts#L972)

***

### httpActionGeneric \{#httpactiongeneric\}

▸ **httpActionGeneric**(`func`): [`PublicHttpAction`](server.md#publichttpaction)

Define una acción HTTP de Convex.

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `func` | (`ctx`: [`GenericActionCtx`](../interfaces/server.GenericActionCtx.md)&lt;[`GenericDataModel`](server.md#genericdatamodel)&gt;, `request`: `Request`) =&gt; `Promise`&lt;`Response`&gt; | La función. Recibe un [GenericActionCtx](../interfaces/server.GenericActionCtx.md) como primer argumento y un objeto `Request` como segundo. |

#### Devuelve \{#returns\}

[`PublicHttpAction`](server.md#publichttpaction)

La función envuelta. Asocia una ruta de URL con esta función en `convex/http.js`.

#### Definido en \{#defined-in\}

[server/impl/registration&#95;impl.ts:467](https://github.com/get-convex/convex-js/blob/main/src/server/impl/registration_impl.ts#L467)

***

### paginationResultValidator \{#paginationresultvalidator\}

▸ **paginationResultValidator**&lt;`T`&gt;(`itemValidator`): [`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

Una función de fábrica de [Validator](values.md#validator) para [PaginationResult](../interfaces/server.PaginationResult.md).

Crea un validador para el resultado de llamar a [paginate](../interfaces/server.OrderedQuery.md#paginate)
con un validador de elementos proporcionado.

Por ejemplo:

```ts
const paginationResultValidator = paginationResultValidator(v.object({
  _id: v.id("users"),
  _creationTime: v.number(),
  name: v.string(),
}));
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `T` | extends [`Validator`](values.md#validator)&lt;[`Value`](values.md#value), `"required"`, `string`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `itemValidator` | `T` | Un validador para cada elemento de la página |

#### Devuelve \{#returns\}

[`VObject`](../classes/values.VObject.md)&lt;&#123; `splitCursor`: `undefined` | `null` | `string` ; `pageStatus`: `undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"` ; `page`: `T`[`"type"`][] ; `continueCursor`: `string` ; `isDone`: `boolean`  &#125;, &#123; `page`: [`VArray`](../classes/values.VArray.md)&lt;`T`[`"type"`][], `T`, `"required"`&gt; ; `continueCursor`: [`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt; ; `isDone`: [`VBoolean`](../classes/values.VBoolean.md)&lt;`boolean`, `"required"`&gt; ; `splitCursor`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `string`, [[`VString`](../classes/values.VString.md)&lt;`string`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt; ; `pageStatus`: [`VUnion`](../classes/values.VUnion.md)&lt;`undefined` | `null` | `"SplitRecommended"` | `"SplitRequired"`, [[`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRecommended"`, `"required"`&gt;, [`VLiteral`](../classes/values.VLiteral.md)&lt;`"SplitRequired"`, `"required"`&gt;, [`VNull`](../classes/values.VNull.md)&lt;`null`, `"required"`&gt;], `"optional"`, `never`&gt;  &#125;, `"required"`, `"page"` | `"continueCursor"` | `"isDone"` | `"splitCursor"` | `"pageStatus"`&gt;

Validador del resultado de la paginación

#### Definido en \{#defined-in\}

[server/pagination.ts:162](https://github.com/get-convex/convex-js/blob/main/src/server/pagination.ts#L162)

***

### httpRouter \{#httprouter\}

▸ **httpRouter**(): [`HttpRouter`](../classes/server.HttpRouter.md)

Devuelve un nuevo objeto [HttpRouter](../classes/server.HttpRouter.md).

#### Devuelve \{#returns\}

[`HttpRouter`](../classes/server.HttpRouter.md)

#### Definido en \{#defined-in\}

[server/router.ts:47](https://github.com/get-convex/convex-js/blob/main/src/server/router.ts#L47)

***

### defineTable \{#definetable\}

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

Define una tabla dentro de un esquema.

Puedes especificar el esquema de tus documentos como un objeto, por ejemplo:

```ts
defineTable({
  field: v.string()
});
```

o como un tipo de esquema, por ejemplo

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DocumentSchema` | extends [`Validator`](values.md#validator)&lt;`Record`&lt;`string`, `any`&gt;, `"required"`, `any`&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | El tipo de documentos que se almacenan en esta tabla. |

#### Devuelve \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;`DocumentSchema`&gt;

Una [TableDefinition](../classes/server.TableDefinition.md) para la tabla.

#### Definido en \{#defined-in\}

[server/schema.ts:593](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L593)

▸ **defineTable**&lt;`DocumentSchema`&gt;(`documentSchema`): [`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

Define una tabla dentro de un esquema.

Puedes especificar el esquema de tus documentos como un objeto, por ejemplo:

```ts
defineTable({
  field: v.string()
});
```

o como un tipo de esquema, por ejemplo

```ts
defineTable(
 v.union(
   v.object({...}),
   v.object({...})
 )
);
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `DocumentSchema` | extends `Record`&lt;`string`, [`GenericValidator`](values.md#genericvalidator)&gt; |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `documentSchema` | `DocumentSchema` | El tipo de documentos que se almacenan en esta tabla. |

#### Devuelve \{#returns\}

[`TableDefinition`](../classes/server.TableDefinition.md)&lt;[`VObject`](../classes/values.VObject.md)&lt;[`ObjectType`](values.md#objecttype)&lt;`DocumentSchema`&gt;, `DocumentSchema`&gt;&gt;

Un [TableDefinition](../classes/server.TableDefinition.md) para la tabla.

#### Definido en \{#defined-in\}

[server/schema.ts:621](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L621)

***

### defineSchema \{#defineschema\}

▸ **defineSchema**&lt;`Schema`, `StrictTableNameTypes`&gt;(`schema`, `options?`): [`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

Define el esquema de este proyecto de Convex.

Esto debe exportarse desde un archivo `schema.ts` en tu directorio `convex/`
del siguiente modo:

```ts
export default defineSchema({
  ...
});
```

#### Parámetros de tipo \{#type-parameters\}

| Nombre | Tipo |
| :------ | :------ |
| `Schema` | extends [`GenericSchema`](server.md#genericschema) |
| `StrictTableNameTypes` | extends `boolean` = `true` |

#### Parámetros \{#parameters\}

| Nombre | Tipo | Descripción |
| :------ | :------ | :------ |
| `schema` | `Schema` | Un mapa de nombre de tabla a [TableDefinition](../classes/server.TableDefinition.md) para todas las tablas de este proyecto. |
| `options?` | [`DefineSchemaOptions`](../interfaces/server.DefineSchemaOptions.md)&lt;`StrictTableNameTypes`&gt; | Configuración opcional. Consulta [DefineSchemaOptions](../interfaces/server.DefineSchemaOptions.md) para una descripción completa. |

#### Devuelve \{#returns\}

[`SchemaDefinition`](../classes/server.SchemaDefinition.md)&lt;`Schema`, `StrictTableNameTypes`&gt;

El esquema.

#### Definido en \{#defined-in\}

[server/schema.ts:769](https://github.com/get-convex/convex-js/blob/main/src/server/schema.ts#L769)