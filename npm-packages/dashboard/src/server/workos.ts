import { NextApiRequest, GetServerSidePropsContext } from "next";
import { WorkOS } from "@workos-inc/node";

let instance: WorkOS | undefined;

export function getWorkOS() {
  if (instance !== undefined) {
    return instance;
  }

  instance = new WorkOS(process.env.WORKOS_API_SECRET, {
    clientId: process.env.WORKOS_CLIENT_ID,
  });
  return instance;
}

export interface WorkOSSession {
  user: {
    id: string;
    email: string;
    emailVerified: boolean;
    firstName?: string;
    lastName?: string;
    profilePictureUrl?: string;
  };
  accessToken?: string;
  impersonator?: {
    email: string;
    reason?: string;
  };
}

export async function getSession(
  req: NextApiRequest | GetServerSidePropsContext["req"],
): Promise<WorkOSSession | null> {
  try {
    const cookieHeader = req.headers.cookie;
    if (!cookieHeader) {
      return null;
    }

    // Extract wos-session cookie
    const sessionCookie = cookieHeader
      .split(";")
      .find((cookie) => cookie.trim().startsWith("wos-session="))
      ?.split("=")[1];

    if (!sessionCookie) {
      return null;
    }

    const workosInstance = getWorkOS();

    // Verify and unseal the session
    const session = workosInstance.userManagement.loadSealedSession({
      sessionData: sessionCookie,
      cookiePassword: process.env.WORKOS_COOKIE_PASSWORD || "",
    });

    if (!session) {
      return null;
    }

    const sess = await session.authenticate();
    if (!sess.authenticated) {
      return null;
    }
    const { user, accessToken } = sess;
    return {
      user: {
        id: user.id,
        email: user.email,
        emailVerified: user.emailVerified,
        firstName: user.firstName || "",
        lastName: user.lastName || "",
        profilePictureUrl: user.profilePictureUrl || "",
      },
      accessToken,
    };
  } catch (error) {
    console.error("Error loading WorkOS session:", error);
    return null;
  }
}

export async function getAccessToken(
  req: NextApiRequest | GetServerSidePropsContext["req"],
): Promise<{ accessToken: string } | null> {
  const session = await getSession(req);
  if (!session || !session.accessToken) {
    return null;
  }

  return { accessToken: session.accessToken };
}

export async function refreshSession(
  req: NextApiRequest,
): Promise<{ accessToken: string } | null> {
  try {
    const session = await getSession(req);
    if (!session) {
      return null;
    }

    return { accessToken: session.accessToken || "" };
  } catch (error) {
    console.error("Error refreshing WorkOS session:", error);
    return null;
  }
}

export interface WithPageAuthRequiredOptions {
  getServerSideProps?: (context: GetServerSidePropsContext) => Promise<any>;
  returnTo?: string;
}

export function withPageAuthRequired(
  options: WithPageAuthRequiredOptions = {},
) {
  return async (context: GetServerSidePropsContext) => {
    const session = await getSession(context.req);

    // If there's a custom getServerSideProps, call it
    if (options.getServerSideProps) {
      const result = await options.getServerSideProps(context);

      // Merge session info with the result
      if (result.props) {
        result.props.session = session;
      }

      return result;
    }

    return {
      props: {
        session,
      },
    };
  };
}
