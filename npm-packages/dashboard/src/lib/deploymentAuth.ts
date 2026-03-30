// Obtain a deploy key to be displayed to the user for them to use
// in machine based workflows like CI/CD.
export const deploymentAuth = async (
  deploymentName: string,
  authHeader: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => deploymentAuthInner(deploymentName, authHeader, "auth");

const deploymentAuthInner = async (
  deploymentName: string,
  authHeader: string,
  authMethod: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => {
  const resp = await fetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/instances/${deploymentName}/${authMethod}`,
    {
      method: "POST",
      headers: { Authorization: authHeader },
    },
  );
  const data = await resp.json();
  if (!resp.ok) {
    return { ok: false, errorCode: data.code, errorMessage: data.message };
  }
  const { adminKey, instanceUrl } = data;
  const deploymentUrl = instanceUrl.endsWith("/")
    ? instanceUrl.slice(0, -1)
    : instanceUrl;
  return { deploymentUrl, adminKey, ok: true };
};
