---
title: "Convex HTTP API"
sidebar_label: "公开 HTTP API"
description: "通过 HTTP 直接连接到 Convex"
---

import Tabs from "@theme/Tabs"; import TabItem from "@theme/TabItem";

用于定义部署的公共函数会通过公开的 HTTP 端点对外暴露。

## Convex 值格式 \{#convex-value-format\}

每个 HTTP API 都带有一个 `format` 查询参数，用于指定文档的格式。目前唯一支持的取值是 `json`。有关详细信息，请参阅我们的[类型页面](/database/types#convex-values)。请注意，为了简化处理，`json` 格式在输入时不支持所有 Convex 数据类型，并且在输出时对多种数据类型使用了重叠的表示方式。我们计划在未来添加一种新的格式，以支持所有 Convex 数据类型。

## API 身份验证 \{#api-authentication\}

Functions API 可以选择通过 `Authorization` 请求头中的 bearer token 以用户身份进行验证。其值为 `Bearer <access_key>`，其中 key 是来自身份认证提供方的 token。有关在 Clerk 中如何工作的详细信息，请参阅 Clerk 文档中的 [under the hood](/auth/clerk#under-the-hood) 部分。

流式导出和流式导入请求需要通过 HTTP `Authorization` 请求头进行部署管理员级别的授权。其值为 `Convex <access_key>`，其中 access key 来自 Convex 仪表盘上的 “Deploy key”，并对你的 Convex 数据提供完整的读写访问权限。

## 函数 API \{#functions-api\}

### POST `/api/query`, `/api/mutation`, `/api/action` \{#post-apiquery-apimutation-apiaction\}

这些 HTTP 端点允许你调用 Convex 函数，并以一个值的形式获取结果。

你可以在仪表盘的
[Settings](/dashboard/deployments/settings.md) 页面找到你的后端部署 URL，然后 API URL 就是
`<CONVEX_URL>/api/query` 等，例如：

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

**JSON 请求体参数**

| Name   | Type   | Required | Description                                                                                                       |
| ------ | ------ | -------- | ----------------------------------------------------------------------------------------------------------------- |
| path   | string | y        | 以字符串形式表示的 Convex 函数路径，格式定义见[此处](/functions/query-functions#query-names)。                     |
| args   | object | y        | 传递给 Convex 函数的具名参数对象。                                                                                |
| format | string | n        | 值的输出格式。有效取值：[`json`]                                                                                   |

**成功时的 Result JSON**

| Field Name | Type         | Description                         |
| ---------- | ------------ | ----------------------------------- |
| status     | string       | &quot;success&quot;                           |
| value      | object       | 按请求格式返回的 Convex 函数结果。  |
| logLines   | list[string] | 函数执行期间打印的日志行。          |

**出错时的 Result JSON**

| Field Name   | Type         | Description                                                                                           |
| ------------ | ------------ | ----------------------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                               |
| errorMessage | string       | 错误消息。                                                                                            |
| errorData    | object       | 如果抛出了[应用错误](/functions/error-handling/application-errors)，则为该错误中包含的错误数据。     |
| logLines     | list[string] | 函数执行期间打印的日志行。                                                                            |

### POST `/api/run/{functionIdentifier}` \{#post-apirunfunctionidentifier\}

此 HTTP 端点允许你通过请求 URL 中的路径调用任意 Convex 函数类型，
并以值的形式获取结果。函数标识符采用字符串格式，
如[此处](/functions/query-functions#query-names)所定义，只是将 `:` 替换为 `/`。

你可以在仪表盘的
[Settings](/dashboard/deployments/settings.md) 页面找到你的后端部署 URL，
对应的 API URL 为 `<CONVEX_URL>/api/run/{functionIdentifier}` 等，例如：

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

**JSON 请求体参数**

| 名称   | 类型   | 必填 | 描述                                              |
| ------ | ------ | ---- | ------------------------------------------------- |
| args   | object | y    | 传递给 Convex 函数的具名参数对象。                |
| format | string | n    | 值的输出格式。默认为 `json`。有效取值：[`json`]  |

**成功时的结果 JSON**

| 字段名   | 类型         | 描述                                           |
| -------- | ------------ | ---------------------------------------------- |
| status   | string       | &quot;success&quot;                                      |
| value    | object       | 按请求格式返回的 Convex 函数结果。            |
| logLines | list[string] | 函数执行期间输出的日志行。                    |

**出错时的结果 JSON**

| 字段名       | 类型         | 描述                                                                                     |
| ------------ | ------------ | ---------------------------------------------------------------------------------------- |
| status       | string       | &quot;error&quot;                                                                                  |
| errorMessage | string       | 错误消息。                                                                               |
| errorData    | object       | 若抛出了[应用错误](/functions/error-handling/application-errors)，则为其中包含的错误数据。 |
| logLines     | list[string] | 函数执行期间输出的日志行。                                                               |