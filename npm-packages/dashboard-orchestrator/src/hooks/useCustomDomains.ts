import useSWR from "swr";
import {
  createCustomDomain,
  createDnsCredential,
  deleteCustomDomain,
  deleteDnsCredential,
  listCustomDomains,
  listDnsCredentials,
  retryCustomDomain,
  verifyCustomDomain,
} from "../lib/orchestratorApi";
import { useAccessToken } from "../lib/useOrchestratorToken";
import { orchestratorUrl } from "../lib/config";

export function useCustomDomains(deploymentId: number | undefined) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { data, error, isLoading, mutate } = useSWR(
    token && deploymentId ? ["customDomains", deploymentId, token] : null,
    () => listCustomDomains(url, token!, deploymentId!),
    // Issuance happens in the background and can take a minute (DNS
    // propagation), so poll while the page is open rather than making the
    // operator reload to watch a domain go active.
    { refreshInterval: 10_000 },
  );

  const add = async (
    domain: string,
    challengeType: "http-01" | "dns-01",
    dnsCredentialId?: number | null,
  ) => {
    if (!token || !deploymentId) return;
    await createCustomDomain(
      url,
      token,
      deploymentId,
      domain,
      challengeType,
      dnsCredentialId,
    );
    await mutate();
  };

  const remove = async (domain: string) => {
    if (!token || !deploymentId) return;
    await deleteCustomDomain(url, token, deploymentId, domain);
    await mutate();
  };

  const retry = async (domain: string) => {
    if (!token || !deploymentId) return;
    await retryCustomDomain(url, token, deploymentId, domain);
    await mutate();
  };

  // Returns the probe result so the caller can surface *why* a domain is
  // still pending — the causes (DNS not pointed here, ACME rate limit) are
  // only fixable by the operator.
  const verify = async (domain: string) => {
    if (!token || !deploymentId) return undefined;
    const result = await verifyCustomDomain(url, token, deploymentId, domain);
    await mutate();
    return result;
  };

  return {
    domains: data?.domains,
    targetHost: data?.targetHost,
    routingEnabled: data?.routingEnabled ?? false,
    providers: data?.providers ?? [],
    error,
    isLoading,
    add,
    remove,
    retry,
    verify,
  };
}

export function useDnsCredentials(teamId: number | undefined) {
  const token = useAccessToken();
  const url = orchestratorUrl();
  const { data, error, isLoading, mutate } = useSWR(
    token && teamId ? ["dnsCredentials", teamId, token] : null,
    () => listDnsCredentials(url, token!, teamId!),
  );

  const add = async (
    name: string,
    provider: string,
    secrets: Record<string, string>,
  ) => {
    if (!token || !teamId) return;
    await createDnsCredential(url, token, teamId, name, provider, secrets);
    await mutate();
  };

  const remove = async (credentialId: number) => {
    if (!token || !teamId) return;
    await deleteDnsCredential(url, token, teamId, credentialId);
    await mutate();
  };

  return {
    credentials: data?.credentials,
    providers: data?.providers ?? [],
    error,
    isLoading,
    add,
    remove,
  };
}
