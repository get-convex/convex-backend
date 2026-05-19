import { useCallback } from "react";
import useSWR from "swr";
import { joinUrlPath } from "@common/lib/helpers/joinUrlPath";

export type AdminKey = {
  id: string;
  name: string;
  creationTime: number;
  revokedTime: number | null;
  isCurrent: boolean;
  /**
   * Last few characters of the underlying admin key, captured at insert /
   * auto-adopt time. Surfaced so the UI can show users which key they are
   * about to revoke without leaking the full secret. `null` for rows that
   * pre-date this field.
   */
  keySuffix: string | null;
};

export type CreatedAdminKey = {
  id: string;
  name: string;
  creationTime: number;
  adminKey: string;
};

/**
 * Hook to manage admin keys against `/api/admin_keys` on a self-hosted
 * deployment.
 *
 * Mirrors the implementation in `dashboard-self-hosted/src/hooks/useAdminKeys`
 * — the orchestrator dashboard delegates admin-key management to the running
 * backend rather than minting its own deploy keys, so we hit exactly the
 * same endpoints. Kept as a sibling copy (instead of importing the
 * self-hosted package) to avoid cross-package coupling between two
 * otherwise-independent Next.js apps in the workspace.
 *
 * `deploymentUrl` + `adminKey` come from `DeploymentInfoContext` (mounted by
 * `OrchestratorDeploymentShell`), where `adminKey` is the deployment's
 * bootstrap key minted by the orchestrator's `/api/dashboard/instances/.../auth`.
 */
export function useAdminKeys({
  deploymentUrl,
  adminKey,
}: {
  deploymentUrl: string;
  adminKey: string;
}) {
  const authHeader = `Convex ${adminKey}`;

  async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
    const url = joinUrlPath(deploymentUrl, path).toString();
    const res = await fetch(url, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        Authorization: authHeader,
        "Convex-Client": "dashboard-0.0.0",
        ...(init.headers ?? {}),
      },
    });
    if (!res.ok) {
      const body = await res.text();
      throw new Error(`${res.status}: ${body}`);
    }
    // Some endpoints (revoke, rename) may return an empty body.
    const text = await res.text();
    return (text ? JSON.parse(text) : (undefined as unknown)) as T;
  }

  const swrKey: [string, string, string] = [
    "admin_keys",
    deploymentUrl,
    adminKey,
  ];
  const { data, error, mutate } = useSWR<AdminKey[]>(swrKey, () =>
    call<AdminKey[]>("/api/admin_keys"),
  );

  const create = useCallback(
    async (name: string) => {
      const created = await call<CreatedAdminKey>("/api/admin_keys", {
        method: "POST",
        body: JSON.stringify({ name }),
      });
      await mutate();
      return created;
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [mutate, deploymentUrl, adminKey],
  );

  const revoke = useCallback(
    async (id: string) => {
      await call(`/api/admin_keys/${encodeURIComponent(id)}/revoke`, {
        method: "POST",
      });
      await mutate();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [mutate, deploymentUrl, adminKey],
  );

  const rename = useCallback(
    async (id: string, name: string) => {
      await call(`/api/admin_keys/${encodeURIComponent(id)}`, {
        method: "PATCH",
        body: JSON.stringify({ name }),
      });
      await mutate();
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [mutate, deploymentUrl, adminKey],
  );

  return {
    keys: data,
    error,
    create,
    revoke,
    rename,
    refresh: mutate,
  };
}
