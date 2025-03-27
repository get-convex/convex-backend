import { Button } from "dashboard-common/elements/Button";
import { useEffect, useState } from "react";
import { useLocalStorage } from "react-use";

export type Deployment = {
  name: string;
  adminKey: string;
  url: string;
};

export function DeploymentList({
  listDeploymentsApiUrl,
  onError,
  onSelect,
}: {
  listDeploymentsApiUrl: string;
  onError: (error: string) => void;
  onSelect: (
    adminKey: string,
    deploymentUrl: string,
    deploymentName: string,
  ) => Promise<void>;
}) {
  const [lastStoredDeployment, setLastStoredDeployment] = useLocalStorage(
    "lastDeployment",
    "",
  );
  const [deployments, setDeployments] = useState<Deployment[]>([]);
  useEffect(() => {
    const f = async () => {
      let resp: Response;
      try {
        resp = await fetch(listDeploymentsApiUrl);
      } catch (e) {
        onError(`Failed to fetch deployments: ${e}`);
        return;
      }
      if (!resp.ok) {
        const text = await resp.text();
        onError(`Failed to fetch deployments: ${resp.statusText} ${text}`);
        return;
      }
      let data: { deployments: Deployment[] };
      try {
        data = await resp.json();
      } catch (e) {
        onError(`Failed to parse deployments: ${e}`);
        return;
      }
      setDeployments(data.deployments);
      const lastDeployment = data.deployments.find(
        (d: Deployment) => d.name === lastStoredDeployment,
      );
      if (lastDeployment) {
        void onSelect(
          lastDeployment.adminKey,
          lastDeployment.url,
          lastDeployment.name,
        );
      }
    };
    void f();
  }, [listDeploymentsApiUrl, onError, onSelect, lastStoredDeployment]);
  return (
    <div className="flex flex-col gap-2">
      <h3>Select a deployment:</h3>
      {deployments.map((d) => (
        <Button
          key={d.name}
          variant="neutral"
          onClick={() => {
            setLastStoredDeployment(d.name);
            void onSelect(d.adminKey, d.url, d.name);
          }}
        >
          {d.name}
        </Button>
      ))}
    </div>
  );
}
