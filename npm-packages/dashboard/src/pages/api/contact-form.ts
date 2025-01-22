import { inspect } from "util";
import {
  PlainClient,
  PlainSDKError,
  ThreadFieldSchemaType,
  UpsertCustomerInput,
} from "@team-plain/typescript-sdk";
import type { NextApiRequest, NextApiResponse } from "next";
import { z } from "zod";
import { auth0 } from "server/auth0";
import { captureException, captureMessage } from "@sentry/nextjs";
import { retryingFetch } from "lib/ssr";
import { Team, ProjectDetails, DeploymentResponse } from "generatedApi";

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

const UserSchema = z.object({
  email: z.string(),
  email_verified: z.boolean(),
  name: z.string().optional(),
  nickname: z.string().optional(),
});

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse<ResponseData>,
) {
  const session = await auth0().getSession(req, res);
  if (!session) {
    captureMessage("No session found");
    return res.status(401).json({ error: "Unauthorized" });
  }

  const { user } = session;

  let validatedUser: z.infer<typeof UserSchema>;
  try {
    validatedUser = UserSchema.parse(user);
  } catch (error: any) {
    captureException(error);
    return res.status(500).json({ error: "Internal Server Error" });
  }

  let body: z.infer<typeof RequestBodySchema>;
  try {
    body = RequestBodySchema.parse(req.body);
  } catch (error: any) {
    return res.status(400).json({ error: error.message });
  }

  const profileDataResp = await retryingFetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/profile`,
    {
      headers: {
        authorization: `Bearer ${session.accessToken}`,
      },
    },
  );
  if (!profileDataResp.ok) {
    const responseText = await profileDataResp.text();
    captureMessage(`Couldn't fetch profile data: ${responseText}`);
    return {
      error: "Internal server Error",
    };
  }

  const { id, email: profileEmail } = await profileDataResp.json();

  const memberDataResp = await retryingFetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/member_data`,
    {
      headers: {
        authorization: `Bearer ${session.accessToken}`,
      },
    },
  );
  if (!memberDataResp.ok) {
    const responseText = await memberDataResp.text();
    captureMessage(`Couldn't fetch member data: ${responseText}`);
    return {
      error: "Internal server Error",
    };
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
    validatedUser,
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
        validatedUser,
      );

      if (upsertCustomerWithEmailIdentifierRes.error) {
        return failedToUpsertPlainCustomer(
          upsertCustomerWithEmailIdentifierRes.error,
          res,
        );
      }
      customerId = upsertCustomerWithEmailIdentifierRes.data.customer.id;
    } else {
      return failedToUpsertPlainCustomer(upsertCustomerRes.error, res);
    }
  } else {
    customerId = upsertCustomerRes.data.customer.id;
  }

  const team = teams.find((t) => t.id === teamId);
  const project = projects.find((p) => p.id === projectId);
  const deployment = deployments.find((d) => d.name === deploymentName);

  const threadFields = [];
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
    console.error(
      inspect(createThreadRes.error, {
        showHidden: false,
        depth: null,
        colors: true,
      }),
    );
    captureMessage(createThreadRes.error.message);
    return res.status(500).json({ error: "Internal Server Error" });
  }

  res.status(200).json({ error: null });
}

function upsertPlainCustomer(
  customerIdentifier: UpsertCustomerInput["identifier"],
  memberId: number,
  profileEmail: string,
  validatedUser: z.infer<typeof UserSchema>,
) {
  return client.upsertCustomer({
    identifier: customerIdentifier,
    onCreate: {
      fullName: validatedUser.name || validatedUser.nickname || profileEmail,
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
        value: validatedUser.name || validatedUser.nickname || profileEmail,
      },
    },
  });
}

function failedToUpsertPlainCustomer(
  error: PlainSDKError,
  res: NextApiResponse,
) {
  console.error(
    inspect(error, {
      showHidden: false,
      depth: null,
      colors: true,
    }),
  );
  captureMessage(error.message);
  return res.status(500).json({ error: "Internal Server Error" });
}
