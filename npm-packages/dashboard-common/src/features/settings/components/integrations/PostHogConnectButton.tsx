import { useState } from "react";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import {
  connectPostHog,
  PostHogProject,
} from "@common/features/settings/lib/posthogOAuth";
import { toast } from "@common/lib/utils";

export function PostHogConnectButton({
  onSelectProject,
}: {
  onSelectProject: (project: PostHogProject) => void;
}) {
  const [isConnecting, setIsConnecting] = useState(false);
  const [projects, setProjects] = useState<PostHogProject[] | null>(null);

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      const found = await connectPostHog();
      if (found.length === 1) {
        onSelectProject(found[0]);
        toast("success", `Loaded project "${found[0].name}" from PostHog`);
        setProjects(null);
      } else {
        setProjects(found);
      }
    } catch (e: any) {
      toast("error", e?.message ?? "PostHog authorization failed");
    } finally {
      setIsConnecting(false);
    }
  };

  if (projects && projects.length > 1) {
    return (
      <div className="flex flex-col gap-1">
        <Combobox<PostHogProject>
          label="PostHog project"
          labelHidden={false}
          placeholder="Select a PostHog project"
          options={projects.map((p) => ({ label: p.name, value: p }))}
          selectedOption={null}
          setSelectedOption={(p) => {
            if (p) {
              onSelectProject(p);
              setProjects(null);
              toast("success", `Loaded project "${p.name}" from PostHog`);
            }
          }}
        />
      </div>
    );
  }

  return (
    <div>
      <Button
        variant="neutral"
        type="button"
        onClick={handleConnect}
        loading={isConnecting}
      >
        Connect with PostHog
      </Button>
    </div>
  );
}
