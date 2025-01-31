import useSWR, { BareFetcher } from "swr";
import { useNents } from "lib/useNents";
import {
  deploymentAuthMiddleware,
  useDeploymentIsDisconnected,
} from "lib/deploymentApi";
import { deploymentFetch } from "lib/fetching";

export function useSourceCode(path: string) {
  const { selectedNent } = useNents();
  const componentQuery = selectedNent ? `&component=${selectedNent.id}` : "";
  const isDisconnected = useDeploymentIsDisconnected();
  const fetcher: BareFetcher = deploymentFetch;
  const { data, error } = useSWR(
    isDisconnected
      ? null
      : `/api/get_source_code?path=${path}${componentQuery}`,
    fetcher,
    {
      use: [deploymentAuthMiddleware],
    },
  );
  if (error) {
    throw error;
  }
  return data as string | null | undefined;
}
