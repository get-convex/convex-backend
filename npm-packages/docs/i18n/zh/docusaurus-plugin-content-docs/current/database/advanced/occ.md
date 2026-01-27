---
title: "OCC 与原子性"
slug: "occ"
hidden: false
sidebar_position: 500
todo: Push under mutations, or inline, or kill (move to Stack)
description:
  "Convex 中的乐观并发控制与事务原子性"
---

在 [查询](/functions/query-functions.mdx) 一节中，我们提到，在 Convex 中使用乐观并发控制（OCC）时，保持确定性至关重要。在本节中，我们将更深入地探讨其中的 *原因*。

## Convex Financial, Inc. \{#convex-financial-inc\}

假设你正在构建一款银行应用，因此数据库中存储了带有余额的账户。你希望用户之间可以互相转账，于是编写了一个变更函数，把资金从一个用户的账户转到另一个用户的账户。

该事务在一次执行中，可能会先读取 Alice 的账户余额，然后再读取 Bob 的账户余额。接着你打算从 Alice 的账户中扣除 5 美元，并将 Bob 的余额增加相同的 5 美元。

下面是我们的伪代码：

```
$14 <- READ Alice
$11 <- READ Bob
WRITE Alice $9
WRITE Bob $16
```

这个账本余额转账是一个典型的数据库场景，需要保证这些写操作只能一起生效。如果只成功了其中一个操作，那就非常糟糕了！

```
$14 <- READ Alice
$11 <- READ Bob
WRITE Alice $9
*crash* // 你的银行损失了 $5
```

你需要一个保证来确保这种情况永远不会发生。你需要事务原子性，而 Convex 能够提供这种保证。

数据正确性的问题要复杂得多。并发事务在读取和编辑相同记录时，会造成*数据竞争*。

在我们的应用中，完全有可能在我们读到 Alice 的余额之后，立刻有人扣减了她的余额。也许她刚在机场用借记卡买了一瓶 3 美元的 Coke Zero。

```
$5 Transfer                           $3 Debit Card Charge
----------------------------------------------------------
$14 <- READ Alice
$11 <- READ Bob
                                        $14 <- READ Alice
                                        WRITE Alice $11
WRITE Alice $9 // 免费可乐！
WRITE Bob $16
```

显然，我们需要防止这类数据竞争发生。我们需要一种方式来处理这些并发冲突。一般来说，有两种常见的方法。

大多数传统数据库选择一种*悲观锁定*策略。（在这里，“悲观”意味着该策略事先假定一定会发生冲突，因此会主动加以预防。）使用悲观锁定时，你首先需要获取 Alice 记录上的锁，然后再获取 Bob 记录上的锁。之后你就可以继续执行事务，因为你可以确定，任何其他需要接触这些记录的事务都会等到你完成且所有写入都提交之后才会继续。

经过数十年的实践，人们已经充分认识到悲观锁定的缺点，而且无可否认。最大的问题来自现实世界中的网络和计算机本身就不可靠。如果持锁方在事务执行到一半时因为某种原因“失踪”了，那么所有想要修改这些记录的其他事务都会一直无限期地等待。非常糟糕！

乐观并发控制，顾名思义，是“乐观”的。它假设事务会成功完成，并且不会预先为任何东西加锁。相当大胆！它怎么能这么自信？

它通过把事务视为一种*声明式提议*来实现这一点：根据读取到的记录版本（“读集”）去写入记录。事务结束时，如果读集中每条记录的版本仍然是该记录的最新版本，那么所有写入就会提交。这意味着没有发生并发冲突。

现在借助我们的版本读集，来看一下 OCC 是如何避免上面那场汽水灾难的：

```
$5 Transfer                           $3 Debit Card Charge
----------------------------------------------------------
(v1, $14) <- READ Alice
(v7, $11) <- READ Bob
                                        (v1, $14) <- READ Alice
                                        WRITE Alice $11
                                        IF Alice.v = v1

WRITE Alice = $9, Bob = $16
    IF Alice.v = v1, Bob.v = v7 // 失败!Alice 为 v2
```

这有点像因为当前不在 HEAD 而无法向 Git 仓库推送。我们都知道，在这种情况下，需要先 pull，然后再做 rebase 或 merge 等操作。

## 当 OCC 失利时，确定性获胜 \{#when-occ-loses-determinism-wins\}

一个最朴素的乐观并发控制方案，会像 Git 一样处理：要求用户或应用自己解决冲突，并判断是否可以安全地重试。

然而在 Convex 中，我们不需要这么做。我们知道这个事务是确定性的（deterministic）。它没有向 Stripe 收费，也没有向文件系统写入任何永久性的值。除了对 Convex 表提出了一些未被应用的原子性更改之外，它根本没有产生任何影响。

这种确定性意味着我们可以直接重新运行该事务；你完全不必担心短暂的数据竞争。如果有必要，我们可以重试多次，直到成功无冲突地执行该事务。

<Admonition type="tip">
  事实上，这个 Git 的类比依然非常贴切。一次 OCC 冲突意味着我们无法 push，因为我们的 HEAD 已经过时了，所以需要对我们的更改做一次 rebase，然后再试一次。而确定性保证了永远不会出现“合并冲突（merge conflict）”，因此（与 Git 不同）这个 rebase 操作最终总能在无需开发者介入的情况下成功。
</Admonition>

## 快照隔离与可串行化 \{#snapshot-isolation-vs-serializability\}

在采用乐观多版本并发控制的数据库中，通常会提供
[快照隔离（snapshot isolation）](https://en.wikipedia.org/wiki/Snapshot_isolation) 的保证。这个
[隔离级别](https://en.wikipedia.org/wiki/Isolation_\(database_systems\))
让人感觉好像所有事务都在数据的某个原子快照上执行，但它容易受到
[异常](https://en.wikipedia.org/wiki/Snapshot_isolation#Definition) 的影响，在某些并发事务组合下可能产生错误结果。Convex 中实现的乐观并发控制则提供了真正的
[可串行化（serializability）](https://en.wikipedia.org/wiki/Serializability)，无论同时发起哪些事务，都能保证得到正确的结果。

## 不需要为此操心 \{#no-need-to-think-about-this\}

这种方法的妙处在于，你可以直接编写你的变更函数，仿佛它们&#95;总会成功&#95;，并且始终能保证原子性执行。

除了出于对 Convex 工作原理的好奇之外，在日常开发中，当你修改表和文档时，大可不必担心冲突、加锁或原子性问题。用“最直观的方式”来编写你的变更函数就能正常工作。