---
name: Convex Destination Setup Guide
title: Convex Destination Connector Setup Guide
description: Read step-by-step instructions on how to connect your Convex deployment as a destination using Fivetran.
menuPosition: 0
---

​

# Convex Setup Guide {% typeBadge connector="convex" /%} {% availabilityBadge connector="convex" /%}

​ Follow our setup guide to connect Fivetran to Convex as a destination. ​

Note that Convex can also be set up as a [source](/docs/databases/convex)

---

​

## Prerequisites

​To connect your Convex deployment to Fivetran, you need the following:
- A [Convex account](https://dashboard.convex.dev)
- A Convex deployment. See [Convex's documentation](https://docs.convex.dev/) to get started.
- Your Convex deployment's URL (e.g., `https://jaded-raven-991.convex.cloud`)
- Your Convex deployment's deploy key. You can find both the deployment URL and deploy key on the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings) page. ​

---

​

## Setup instructions

​

### <span class="step-item">Locate your Deployment Credentials</span>

1. Navigate to your deployment on the [Convex Dashboard](https://dashboard.convex.dev/).​
2. Navigate to the [Production Deployment Settings](https://docs.convex.dev/dashboard/deployments/deployment-settings).
3. Find your deployment URL and deploy key and make a note of them. You will need them to configure Fivetran.

### <span class="step-item">Setup Fivetran Destination</span>

1. Log into your fivetran account
2. Go to the destinations page and click "Add Destination".
3. Select Convex as the destination type
4. Enter your deployment credentials.
5. Click **Save & Test**. Fivetran tests and validates connection to your Convex deployment. ​

### Configuring the `fivetran_metadata` connector

### Creating a new connector

1. From fivetran dashboard, click Connectors -> Add Connector
2. Select and configure a data source of your choice.
3. Select your Convex destination that you created above.
4. Start your initial sync by clicking "Sync Now".
5. The initial sync may fail with a user action to update [convex/schema.ts](https://docs.convex.dev/database/schemas). Follow the instructions in the error message to update your `schema.ts`. The error message will give you code to copy-paste into your `schema.ts`. It may take around 20 minutes for the error to appear.
6. Deploy your `schema.ts` with `npx convex deploy`.
7. Retry your initial sync by clicking "Sync Now"

A sample schema.ts for a source with a single table `email.cars` - may look like this.
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

​
[<i aria-hidden="true" class="material-icons">description</i> Destination Connector Overview](/docs/destinations/convex_destination)
[<i aria-hidden="true" class="material-icons">description</i> Source Connector Overview](/docs/databases/convex)
​ <b> </b> ​
[<i aria-hidden="true" class="material-icons">home</i> Documentation Home](/docs/getting-started)
