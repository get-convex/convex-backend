import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { toast } from "@common/lib/utils";
import { createDeploymentClient } from "@convex-dev/platform";
import {
  CreateLogStreamArgs,
  UpdateLogStreamArgs,
} from "@convex-dev/platform/deploymentApi";
import { useAdminKey, useDeploymentUrl } from "./deploymentApi";

export function useCreateLogStream(): (
  args: CreateLogStreamArgs,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (args: CreateLogStreamArgs) => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/create_log_stream", {
      body: args,
    });
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("POST", response.url, err);
      toast("error", err.message);
    }
  };
}

export function useUpdateLogStream(): (
  id: string,
  update: UpdateLogStreamArgs,
) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (id: string, update: UpdateLogStreamArgs) => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/update_log_stream/{id}", {
      params: {
        path: {
          id,
        },
      },
      body: update,
    });
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("DELETE", response.url, err);
      toast("error", err.message);
    }
  };
}

export function useDeleteLogStream(): (id: string) => Promise<void> {
  const { reportHttpError } = useContext(DeploymentInfoContext);
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();

  return async (id: string) => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/delete_log_stream/{id}", {
      params: {
        path: {
          id,
        },
      },
    });
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("DELETE", response.url, err);
      toast("error", err.message);
    }
  };
}

export function useRotateWebhookSecret(): (id: string) => Promise<void> {
  const deploymentUrl = useDeploymentUrl();
  const adminKey = useAdminKey();
  const { reportHttpError } = useContext(DeploymentInfoContext);

  return async (id: string) => {
    const client = createDeploymentClient(deploymentUrl, adminKey);
    const { response } = await client.POST("/rotate_webhook_secret/{id}", {
      params: {
        path: {
          id,
        },
      },
    });
    if (response.status !== 200) {
      const err = await response.json();
      reportHttpError("POST", response.url, err);
      toast("error", err.message);
    }
  };
}
