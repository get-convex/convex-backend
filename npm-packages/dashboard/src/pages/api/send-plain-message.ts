import {
  PlainClient,
  ThreadFieldSchemaType,
  UpsertCustomerInput,
} from "@team-plain/typescript-sdk";
import type { NextApiRequest, NextApiResponse } from "next";
import { z } from "zod";
import { captureException, captureMessage } from "@sentry/nextjs";
import { Team, ProjectDetails, DeploymentResponse } from "generatedApi";
import { retryingFetch } from "lib/ssr";

const apiKey = process.env.PLAIN_API_KEY;

if (!apiKey) {
  throw new Error("PLAIN_API_KEY environment variable is not set");
}

const client = new PlainClient({
  apiKey,
});

export type ResponseData = {
  error: string | null;
};

const RequestBodySchema = z.object({
  subject: z.string(),
  message: z.string(),
  teamId: z.number(),
  projectId: z.number().optional(),
  deploymentName: z.string().optional(),
});

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse<ResponseData>,
) {
  if (req.headers["x-plain-api-key"] !== process.env.PLAIN_API_KEY) {
    return res.status(401).json({ error: "Unauthorized" });
  }

  let body: z.infer<typeof RequestBodySchema>;
  try {
    body = RequestBodySchema.parse(req.body);
  } catch (error: any) {
    return res.status(400).json({ error: error.message });
  }

  try {
    const profileDataResp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/profile`,
      {
        headers: {
          authorization: `Bearer ${req.headers["x-convex-access-token"]}`,
        },
      },
    );
    if (!profileDataResp.ok) {
      const responseText = await profileDataResp.text();
      throw new Error(`Couldn't fetch profile data: ${responseText}`);
    }

    const {
      id,
      email: profileEmail,
      name: profileName,
    } = await profileDataResp.json();

    const memberDataResp = await retryingFetch(
      `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/member_data`,
      {
        headers: {
          authorization: `Bearer ${req.headers["x-convex-access-token"]}`,
        },
      },
    );
    if (!memberDataResp.ok) {
      const responseText = await memberDataResp.text();
      throw new Error(`Couldn't fetch member data: ${responseText}`);
    }
    const {
      teams,
      projects,
      deployments,
    }: {
      teams: Team[];
      projects: ProjectDetails[];
      deployments: DeploymentResponse[];
    } = await memberDataResp.json();
    const { teamId, projectId, deploymentName } = body;

    let customerId: string | null = null;

    const upsertCustomerRes = await upsertPlainCustomer(
      {
        externalId: id.toString(),
      },
      id,
      profileEmail,
      profileName,
    );

    if (upsertCustomerRes.error) {
      const customerAlreadyExists =
        upsertCustomerRes.error.type === "mutation_error" &&
        upsertCustomerRes.error.errorDetails.code ===
          "customer_already_exists_with_email";
      if (customerAlreadyExists) {
        const upsertCustomerWithEmailIdentifierRes = await upsertPlainCustomer(
          { emailAddress: profileEmail },
          id,
          profileEmail,
          profileName,
        );

        if (upsertCustomerWithEmailIdentifierRes.error) {
          throw new Error(
            `Failed to upsert customer: ${upsertCustomerWithEmailIdentifierRes.error.message}`,
          );
        }
        customerId = upsertCustomerWithEmailIdentifierRes.data.customer.id;
      } else {
        throw new Error(
          `Failed to upsert customer: ${upsertCustomerRes.error.message}`,
        );
      }
    } else {
      customerId = upsertCustomerRes.data.customer.id;
    }

    for (const team of teams) {
      await upsertPlainTenant(
        client,
        team,
        req.headers["x-convex-access-token"] as string,
      );
    }

    await setPlainCustomerTenants(client, customerId, teams);

    const team = teams.find((t) => t.id === teamId);
    const project = projects.find((p) => p.id === projectId);
    const deployment = deployments.find((d) => d.name === deploymentName);

    const threadFields: Array<{
      key: string;
      stringValue: string;
      type: ThreadFieldSchemaType;
    }> = [];

    if (team) {
      threadFields.push({
        key: "team_id",
        stringValue: team.id.toString(),
        type: ThreadFieldSchemaType.String,
      });
      threadFields.push({
        key: "team_slug",
        stringValue: team.slug,
        type: ThreadFieldSchemaType.String,
      });
    }

    if (project) {
      threadFields.push({
        key: "project_id",
        stringValue: project.id.toString(),
        type: ThreadFieldSchemaType.String,
      });
      threadFields.push({
        key: "project_slug",
        stringValue: project.slug,
        type: ThreadFieldSchemaType.String,
      });
    }

    if (deployment) {
      threadFields.push({
        key: "deployment_name",
        stringValue: deployment.name,
        type: ThreadFieldSchemaType.String,
      });
    }

    const createThreadRes = await client.createThread({
      customerIdentifier: {
        customerId,
      },
      title: body.subject,
      threadFields,
      components: [
        {
          componentText: {
            text: body.message,
          },
        },
      ],
    });

    if (createThreadRes.error) {
      throw new Error(
        `Failed to create thread: ${createThreadRes.error.message}`,
      );
    }

    res.status(200).json({ error: null });
  } catch (error: any) {
    captureException(error, {
      extra: {
        requestBody: body,
      },
    });
    return res.status(500).json({ error: "Internal Server Error" });
  }
}

function upsertPlainCustomer(
  customerIdentifier: UpsertCustomerInput["identifier"],
  memberId: number,
  profileEmail: string,
  profileName: string | null,
) {
  return client.upsertCustomer({
    identifier: customerIdentifier,
    onCreate: {
      fullName: profileName || profileEmail,
      externalId: memberId.toString(),
      email: {
        email: profileEmail,
        isVerified: true,
      },
    },
    onUpdate: {
      email: {
        email: profileEmail,
        isVerified: true,
      },
      fullName: {
        value: profileName || profileEmail,
      },
    },
  });
}

async function upsertPlainTenant(
  plainClient: PlainClient,
  team: Team,
  accessToken: string,
) {
  const upsertTenantRes = await plainClient.upsertTenant({
    identifier: {
      externalId: team.id.toString(),
    },
    externalId: team.id.toString(),
    name: team.name,
    url: { value: null },
  });

  if (upsertTenantRes.error) {
    captureMessage(
      `Couldn't upsert tenant: ${upsertTenantRes.error.message}`,
      "error",
    );
    return;
  }

  const subscriptionResp = await retryingFetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/teams/${team.id}/get_orb_subscription`,
    {
      headers: {
        authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!subscriptionResp.ok) {
    const responseText = await subscriptionResp.text();
    captureMessage(`Couldn't fetch subscription: ${responseText}`, "error");
    return;
  }

  let tier = "CONVEX_BASE";
  try {
    const subscription = await subscriptionResp.json();
    if (subscription && subscription.status === "active") {
      const { planType } = subscription.plan;
      tier = planType;
    }
  } catch (error) {
    // Do nothing
  }

  const updateTenantTierRes = await plainClient.updateTenantTier({
    tenantIdentifier: {
      externalId: team.id.toString(),
    },
    tierIdentifier: {
      externalId: tier,
    },
  });

  if (updateTenantTierRes.error) {
    captureMessage(
      `Couldn't update tenant tier: ${updateTenantTierRes.error.message}`,
      "error",
    );
  }
}

async function setPlainCustomerTenants(
  plainClient: PlainClient,
  customerId: string,
  teams: Team[],
) {
  const setCustomerTenantsRes = await plainClient.setCustomerTenants({
    customerIdentifier: {
      customerId,
    },
    tenantIdentifiers: teams.map((t) => ({
      externalId: t.id.toString(),
    })),
  });

  if (setCustomerTenantsRes.error) {
    captureMessage(
      `Couldn't set customer tenants: ${setCustomerTenantsRes.error.message}`,
      "error",
    );
  }
}
