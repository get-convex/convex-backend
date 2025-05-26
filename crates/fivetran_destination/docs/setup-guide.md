---
name: Setup Guide
title: Fivetran for Convex | Destination Setup Guide
description: Read step-by-step instructions on how to connect your Convex deployment as a destination using Fivetran.
menuPosition: 0
hidden: true
---

# Convex Setup Guide {% badge text="Partner-Built" /%} {% availabilityBadge connector="convex_destination" /%}

Follow our setup guide to connect Fivetran to Convex as a destination. 

> NOTE: This destination is [partner-built](/docs/partner-built-program). For any questions related to the Convex destination and its documentation, refer to Convex's support team. For SLA details, see [Convex's Status and Guarantees documentation](https://docs.convex.dev/production/state).

---

## Prerequisites

To connect your Convex deployment to Fivetran, you need the following:
- A [Convex account](https://dashboard.convex.dev).
- A Convex deployment. See [Convex's documentation](https://docs.convex.dev/) to get started.
- Your Convex deployment's URL (e.g., `https://jaded-raven-991.convex.cloud`).
- Your Convex deployment's deploy key. You can find both the deployment URL and deploy key on the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings) page. 

---

## Setup instructions

### <span class="step-item">Find your deployment credentials</span>

1. Go to your deployment on the [Convex Dashboard](https://dashboard.convex.dev/).
2. Go to the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings).
3. Find your **deployment URL** and **deploy key** and make a note of them. You will need them to configure Fivetran.

### <span class="step-item">Complete Fivetran configuration</span>

1. Log in to your [Fivetran account](https://fivetran.com/login).
2. Go to the **Destinations** page and click **Add destination**.
3. Enter a **Destination name** of your choice and then click **Add**.
4. Select Convex as the destination type.
5. Enter your [deployment credentials](#findyourdeploymentcredentials).
6. Click **Save & Test**. Fivetran tests and validates connection to your Convex deployment.

### <span class="step-item">Using Convex destination with a source connector</span>

1. In your Fivetran dashboard, click **Connectors > Add Connector**.
2. Select and configure a data source of your choice.
3. Select the [Convex destination you connected to](#completefivetranconfiguration).
4. Start your initial sync by clicking **Sync Now**.
5. The initial sync may fail with a user action to update [convex/schema.ts](https://docs.convex.dev/database/schemas).
   Follow the instructions in the error message to update your `schema.ts`. The error message will give you code to
   copy-paste into your `schema.ts`. It may take around 20 minutes for the error to appear.
6. Deploy your `schema.ts` with `npx convex deploy`.
7. Retry your initial sync by clicking **Sync Now**.

The example below is a sample `schema.ts` for a source with a single table `email.cars`.
```typescript
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

const fivetranTables = {
  email_cars: defineTable({
    description: v.union(v.string(), v.null()),
    fivetran: v.object({
      columns: v.object({
        directory: v.union(v.string(), v.null()),
        file: v.union(v.string(), v.null()),
        line: v.union(v.int64(), v.null()),
        modified: v.union(v.float64(), v.null()),
      }),
      synced: v.float64(),
    }),
    make: v.union(v.string(), v.null()),
    model: v.union(v.string(), v.null()),
    price: v.union(v.float64(), v.null()),
    year: v.union(v.int64(), v.null()),
  })
    .index("by_fivetran_synced", ["fivetran.synced"])
    .index("by_primary_key", [
      "fivetran.columns.directory",
      "fivetran.columns.file",
      "fivetran.columns.line",
      "fivetran.columns.modified",
    ]),
};

export default defineSchema({
  ...fivetranTables,
});
```

---

## Related articles

[<i aria-hidden="true" class="material-icons">description</i> Destination Overview](/docs/destinations/convex)

<b> </b>

[<i aria-hidden="true" class="material-icons">home</i> Documentation Home](/docs/getting-started)
