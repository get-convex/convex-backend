---
title: "Health"
slug: "health"
sidebar_position: 0
---

The [health page](https://dashboard.convex.dev/deployment/) is the landing page
for your deployment. On this page, you can see some important information about
the health of your deployment.

## Failure Rate

![Failure Rate Card](/screenshots/health_failure_rate.png)

The failure rate card shows the percentage of failed request by minute ove the
last hour. The failure rate is calculated as the number of failed requests
divided by the total number of requests.

## Cache Hit Rate

![Cache Hit Rate Card](/screenshots/health_cache_hit_rate.png)

The cache hit rate card shows the percentage of cache hits by minute over the
last hour. The cache hit rate is calculated as the number of cache hits divided
by the total number of requests.

Cache hit rate only applies to query functions.

## Scheduler Status

![Scheduler Status Card](/screenshots/scheduler_overdue.png)

The scheduler status card shows the status of the
[scheduler](/scheduling/scheduled-functions). If the scheduler falls behind due
to too many scheduled tasks, the status will show as "Overdue", displaying the
lag time in minutes.

You may click the button in the top right corner of the card to view a chart
showing the scheduler status over the last hour.

![Scheduler Status Chart](/screenshots/scheduler_status.png)

## Last Deployed

![Last Deployed Card](/screenshots/health_last_deployed.png)

The last deployed card shows the time of the last time your functions were
deployed.

## Integrations

<Admonition type="info">

Integrations are only available for paid teams.

</Admonition>

![Last Deployed Card](/screenshots/health_integrations.png)

The integrations card shows the status of your
[Exception Reporting](/production/integrations/exception-reporting) and
[Log Streams](/production/integrations/log-streams) integrations, with quick
links to view and configure your integrations.

## Insights

![Insights Card](/screenshots/insights.png)

The Health page also surfaces insights about your deployment, with suggestions
on how to improve performance and reliability.

Each Insight contains a description of the issue, the impact on your deployment
(via a chart and event log), and a link to learn more about the issue and how to
resolve it.

Clicking on an Insight will open a breakdown of the issue, including a larger
chart and a list of events that triggered the Insight.

![Insight Breakdown](/screenshots/insights_breakdown.png)

Available insights include:

- Functions that are
  [reading too many bytes](/production/state/limits#transactions) in a single
  transaction.
- Functions that are
  [reading too many documents](/production/state/limits#transactions) in a
  single transaction.
- Functions that are experiencing [write conflicts](/error#1).
