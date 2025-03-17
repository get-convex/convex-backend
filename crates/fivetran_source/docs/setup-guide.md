---
name: Convex Database Connector Setup Guide
title: Convex Database Connector Setup Guide
description: Read step-by-step instructions on how to connect your Convex deployment with your destination using Fivetran connectors.
menuPosition: 0
---

​

# Convex Setup Guide {% typeBadge connector="convex" /%} {% availabilityBadge connector="convex" /%}

​ Follow our setup guide to connect Convex to Fivetran. ​

Note that Convex can also be set up as a [destination](/docs/destinations/convex)

---

​

## Prerequisites

​To connect your Convex deployment to Fivetran, you need the following:
- A Convex account on a [Professional plan](https://www.convex.dev/pricing)
- A Convex deployment. See [Convex's documentation](https://docs.convex.dev/) to get started.
- Your Convex deployment's URL (e.g., `https://jaded-raven-991.convex.cloud`)
- Your Convex deployment's deploy key. You can find both the deployment URL and deploy key on the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings) page. ​

---

​

## Setup instructions

​

> IMPORTANT: You must have a [Convex Professional plan](https://www.convex.dev/pricing) to be able use the Convex connector. ​

### <span class="step-item">Locate your Deployment Credentials</span>

1. Navigate to your deployment on the [Convex Dashboard](https://dashboard.convex.dev/).​
2. Navigate to the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings).
3. Find your deployment URL and deploy key and make a note of them. You will need them to configure Fivetran.

### <span class="step-item">Finish Fivetran configuration</span>

1. In your [connector setup form](/docs/getting-started/fivetran-dashboard/connectors#addanewconnector), enter a destination schema prefix. This prefix applies to each replicated schema and cannot be changed once your connector is created. ​
2. Select Convex as your source connector.
3. Enter your deployment credentials.
4. Click **Save & Test**. Fivetran tests and validates our connection to your Convex deployment. Upon successful completion of the setup tests, you can sync your data using Fivetran. ​

### Setup tests

Fivetran performs the following tests to ensure that we can connect to your Convex deployment.

- Validating that your deployment credentials.
- Ensuring you are on a [Convex Professional plan](https://www.convex.dev/pricing).

---

## Related articles

​
[<i aria-hidden="true" class="material-icons">description</i> Source Connector Overview](/docs/databases/convex)
[<i aria-hidden="true" class="material-icons">description</i> Destination Connector Overview](/docs/destinations/convex_destination)
​ <b> </b> ​
[<i aria-hidden="true" class="material-icons">home</i> Documentation Home](/docs/getting-started)
