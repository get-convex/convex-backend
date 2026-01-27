---
title: "数据"
slug: "data"
sidebar_position: 5
description:
  "在仪表盘中查看、编辑和管理数据库表和文档"
---

![数据仪表盘页面](/screenshots/data.png)

[数据页面](https://dashboard.convex.dev/deployment/data)允许你查看并管理所有的表和文档。

页面左侧是你的表列表。点击某个表后，你可以在该表中创建、查看、更新和删除文档。

你可以拖拽每个表中的列标题，以直观地重新排列数据的显示顺序。

你也可以通过[命令行界面（CLI）](/cli.md#display-data-from-tables)以只读方式查看数据页面。

```sh
npx convex data [table]
```

## 筛选文档 \{#filtering-documents\}

你可以在数据页面中点击页面顶部的“Filter”按钮来筛选文档。

![Data filters](/screenshots/data_filters.png)

文档中的所有字段都可以使用 Convex 查询语法所支持的操作进行筛选。[相等](/database/reading-data/filters.mdx#equality-conditions)
和[比较](/database/reading-data/filters.mdx#comparisons)在仪表盘中筛选时与使用 Convex 客户端进行查询时遵循相同的规则。
你也可以根据字段的类型进行筛选。

要添加筛选条件，点击现有筛选条件旁边的 `+`。如果你添加了多个条件，它们会通过 `and` 运算进行组合判断。

对于每个筛选条件，你必须选择要筛选的字段、运算类型以及比较值。在第三个输入框（选择值）中，你可以输入一个有效的 Convex
值，比如 `"a string"`、`123`，或者甚至是一个复杂对象，比如 `{ a: { b: 2 } }`。

<Admonition type="note">
  当按 `_creationTime` 进行筛选时，会显示一个日期选择器，而不是普通的 JavaScript 语法输入框。对 `_creationTime` 的比较是以纳秒为粒度进行的，因此如果你想筛选到一个精确的时间点，可以尝试添加两个筛选条件：
  `creationTime >= $time` 和 `creationTime <= $time + 1 minute`。
</Admonition>

## 编写自定义查询 \{#writing-custom-queries\}

你可以直接在仪表盘中编写[查询](/database/reading-data/reading-data.mdx)。这样可以对数据执行任意过滤和转换操作，包括排序、连接、分组和聚合。

在数据页面顶部的 `⋮` 更多操作菜单中，点击 “Custom query” 选项。

<img src="/screenshots/data_custom_query.png" alt="Custom query 按钮" width={250} />

这会打开与
[running your deployed functions](/dashboard/deployments/functions.md#running-functions)
相同的 UI，只是会选中 “Custom test query” 选项，这样你就可以编辑查询的源代码。该源代码会被发送到你的部署中，并在你点击 “Run Custom Query” 按钮时执行。

![运行自定义测试查询](/screenshots/data_custom_query_runner.png)

如果你不在数据页面，也可以通过所有部署页面右下角显示的固定 *fn* 按钮打开这个 UI。打开函数运行器的键盘快捷键是 Ctrl + `（反引号）。

## 创建表 \{#creating-tables\}

在仪表盘中点击“Create Table”按钮，并输入表的新名称即可创建表。

## 创建文档 \{#creating-documents\}

你可以使用数据表工具栏中的 “Add Documents” 按钮向表中添加单个文档。

点击 “Add Documents” 后，会打开一个侧边栏，你可以使用 JavaScript 语法向表中添加新文档。要一次性添加多个文档，请在编辑器中的数组中添加新的对象。

![Add document](/screenshots/data_add_document.png)

## 快捷操作（上下文菜单） \{#quick-actions-context-menu\}

你可以右键单击文档或值，打开包含快捷操作的上下文菜单，例如复制值、按所选值快速筛选以及删除文档。

![Quick actions context menu](/screenshots/data_context_menu.png)

## 编辑单元格 \{#editing-a-cell\}

要编辑单元格的值，在数据表中双击该单元格，或者在单元格被选中时按 Enter 键。你可以使用方向键切换当前选中的单元格。

你可以直接在单元格内修改值，然后按 Enter 保存。

<Admonition type="note">
  你甚至可以在这里编辑值的类型，只要它满足你的
  [模式](/database/schemas.mdx) —— 比如试试把一个字符串替换成一个对象！
</Admonition>

![Inline value editor](/screenshots/data_edit_inline.png)

## 编辑文档 \{#editing-a-document\}

要同时编辑文档中的多个字段，将鼠标悬停在该文档上，然后单击右键以打开上下文菜单。接着点击“Edit Document”。

![编辑整个文档](/screenshots/data_edit_document.png)

## 向其他文档添加引用 \{#adding-references-to-other-documents\}

要引用另一份文档，请使用你想要引用的文档的字符串 ID。

你可以通过点击该文档所在的单元格并按下 CTRL/CMD+C 来复制该 ID。

## 批量编辑文档 \{#bulk-editing-documents\}

你可以一次编辑多个或所有文档。要选择所有文档，点击表头行中的复选框。要选择单个文档，将鼠标悬停在最左侧的单元格上，然后点击出现的复选框。要一次选择多个相邻的文档，按住 Shift 键的同时点击复选框。

当至少选中一个文档后，表格工具栏中会显示“(Bulk) Edit Document(s)”按钮。点击该按钮，右侧会出现一个编辑器。

![批量编辑文档](/screenshots/data_bulk_edit.png)

## 删除文档 \{#deleting-documents\}

当你至少选择了一个文档（见上文）时，“Delete Document(s)” 按钮会出现在表格工具栏中。点击该按钮即可删除文档。如果你在生产部署环境中编辑数据，在删除文档之前会先弹出一个确认对话框。

## 清空数据表 \{#clear-a-table\}

你也可以在数据页面顶部点击 `⋮` 更多菜单，然后选择 &quot;Clear Table&quot; 来删除所有文档。此操作会删除表中的所有文档，但不会删除表本身。

在生产环境中，Convex 仪表盘会要求你在删除前先输入该表的名称。

## 删除表 \{#delete-a-table\}

<Admonition type="caution" title="这是一个永久性操作">
  删除表是不可逆的。在生产环境中，Convex 仪表盘会要求你在删除前输入该表的名称。
</Admonition>

可以通过点击数据页面顶部的 `⋮` 更多菜单找到 “Delete table” 按钮。此操作会删除该表中的所有文档，并将该表从你的表列表中移除。如果此表包含索引，你需要重新部署你的 Convex 函数（分别通过运行 `npx convex deploy` 或 `npx convex dev`，用于生产环境或开发环境）来重新创建这些索引。

## 生成模式 \{#generating-a-schema\}

在页面左下角有一个 “Generate Schema” 按钮，你可以点击它，让 Convex 为此表中的所有文档生成一个[模式](/database/schemas.mdx)。

![Generate Schema button](/screenshots/data_generate_schema.png)

## 查看表的模式 \{#view-the-schema-of-a-table\}

可以通过点击数据页面顶部的 `⋮` 省略号菜单中的 “Schema” 按钮来找到它。

此按钮会打开一个面板，显示与所选表关联的已保存和自动生成的
[模式](/database/schemas.mdx)。

## 查看表的索引 \{#view-the-indexes-of-a-table\}

在数据页面顶部，点击 `⋮` 更多选项菜单，可以找到 “Indexes” 按钮。

点击该按钮会打开一个面板，显示与所选表关联的
[indexes](/database/reading-data/indexes/indexes.md)。

尚未完成回填的索引，其名称旁会显示加载中的旋转图标。