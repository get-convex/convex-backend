---
name: Setup Guide
title: Convex connector by Fivetran | Setup Guide
description: Read step-by-step instructions on how to connect your Convex deployment with your destination using Fivetran connectors.
menuPosition: 0
---


# Convex Setup Guide {% badge text="Partner-Built" /%} {% availabilityBadge connector="convex" /%}

Follow the setup guide to connect Convex to Fivetran.

> NOTE: This connector is [partner-built](/docs/partner-built-program). For any questions related to Convex connector and its documentation, contact Convex's support team. For details on SLA, see [Convex's Status and Guarantees documentation](https://docs.convex.dev/production/state).

---

## Prerequisites

To connect your Convex deployment to Fivetran, you need the following:
- A Convex account on the [Professional plan](https://www.convex.dev/plans).
- A Convex deployment. See [Convex's documentation](https://docs.convex.dev/) to get started.
- Your Convex deployment's URL (for example, `https://jaded-raven-991.convex.cloud`).
- Your Convex deployment's deploy key. You can find both the deployment URL and deploy key on the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings) page.

---

## Setup instructions

### <span class="step-item">Find your deployment credentials</span>

1. Go to your deployment on the [Convex dashboard](https://dashboard.convex.dev/).
2. Go to the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings).
3. Find your deployment URL and deploy key and make a note of them. You will need them to configure Fivetran.

### <span class="step-item">Finish Fivetran configuration</span>

1. In your [connection setup form](/docs/using-fivetran/fivetran-dashboard/connectors#addanewconnection), enter a **Destination schema prefix**. This prefix applies to each replicated schema and cannot be changed once your connection is created.
2. Enter the **Deployment URL** you found.
3. Enter your **Deploy Key** you found.
4. Click **Save & Test**. Fivetran tests and validates our connection to your Convex deployment. Upon successful completion of the setup tests, you can sync your data using Fivetran.

### Setup tests

Fivetran performs the following tests to ensure that we can connect to your Convex deployment.

- Validating that your deployment credentials.
- Ensuring you are on a [Convex Professional plan](https://www.convex.dev/plans).

---

## Related articles

[<i aria-hidden="true" class="material-icons">description</i> Connector Overview](/docs/connectors/databases/convex)

<b> </b>

[<i aria-hidden="true" class="material-icons">account_tree</i> Schema Information](/docs/connectors/databases/convex#schemainformation)

<b> </b>

[<i aria-hidden="true" class="material-icons">home</i> Documentation Home](/docs/getting-started)
