---
id: "react.Watch"
title: "接口：Watch<T>"
custom_edit_url: null
---

[react](../modules/react.md).Watch

对 Convex 查询函数输出结果的监听。

## 类型参数 \{#type-parameters\}

| 名称 |
| :------ |
| `T` |

## 方法 \{#methods\}

### onUpdate \{#onupdate\}

▸ **onUpdate**(`callback`): () =&gt; `void`

对查询结果发起监听。

这将订阅该查询，并在查询结果发生变化时调用回调函数。

**重要：如果客户端已经使用相同参数订阅了该查询，
在查询结果被更新之前，此回调不会被调用。** 如需获取当前的本地结果，请调用
[localQueryResult](react.Watch.md#localqueryresult)。

#### 参数 \{#parameters\}

| 名称 | 类型 | 描述 |
| :------ | :------ | :------ |
| `callback` | () =&gt; `void` | 每当查询结果发生变化时调用的函数。 |

#### 返回值 \{#returns\}

`fn`

* 用于取消此订阅的函数。

▸ (): `void`

开始监听某个查询的输出结果。

这会订阅该查询，并在查询结果发生变化时调用回调函数。

**重要：如果客户端已经使用相同参数订阅了该查询，那么在查询结果更新之前，此回调不会被调用。** 若要获取当前的本地结果，请调用
[localQueryResult](react.Watch.md#localqueryresult)。

##### 返回值 \{#returns\}

`void`

* 一个用于释放该订阅的函数。

#### 定义于 \{#defined-in\}

[react/client.ts:170](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L170)

***

### localQueryResult \{#localqueryresult\}

▸ **localQueryResult**(): `undefined` | `T`

获取当前查询的结果。

仅当客户端已经订阅了该查询并且已经从服务器收到了结果，或该查询的值已通过乐观方式设置时，才会返回结果。

**`Throws`**

如果查询在服务器上遇到错误，则抛出错误。

#### 返回值 \{#returns\}

`undefined` | `T`

查询的结果，如果当前未知则为 `undefined`。

#### 定义于 \{#defined-in\}

[react/client.ts:182](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L182)

***

### journal \{#journal\}

▸ **journal**(): `undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

获取此查询的当前 [QueryJournal](../modules/browser.md#queryjournal)。

如果我们尚未收到此查询的结果，则返回 `undefined`。

#### 返回值 \{#returns\}

`undefined` | [`QueryJournal`](../modules/browser.md#queryjournal)

#### 定义于 \{#defined-in\}

[react/client.ts:194](https://github.com/get-convex/convex-js/blob/main/src/react/client.ts#L194)