import { useState } from "react";
import { Button } from "@ui/Button";
import { Combobox } from "@ui/Combobox";
import {
  connectPostHog,
  PostHogProject,
} from "@common/features/settings/lib/posthogOAuth";
import { PostHogLogo } from "@common/lib/logos/PostHogLogo";
import { toast } from "@common/lib/utils";

// Shown on both cloud and self-hosted Convex dashboards. The flow targets
// PostHog Cloud (us/eu). Users self-hosting PostHog can ignore this and
// continue with manual entry — we cannot detect a self-hosted PostHog from
// the Convex side, so we never hide the button.
export function PostHogConnectButton({
  onSelectProject,
}: {
  onSelectProject: (project: PostHogProject) => void;
}) {
  const [isConnecting, setIsConnecting] = useState(false);
  const [projects, setProjects] = useState<PostHogProject[] | null>(null);
  const [selectedProject, setSelectedProject] = useState<PostHogProject | null>(
    null,
  );

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      const found = await connectPostHog();
      setProjects(found);
      if (found.length === 1) {
        setSelectedProject(found[0]);
        onSelectProject(found[0]);
        toast("success", `Loaded project "${found[0].name}" from PostHog`);
      } else {
        setSelectedProject(null);
      }
    } catch (e: unknown) {
      const message =
        e instanceof Error ? e.message : "PostHog authorization failed";
      toast("error", message);
    } finally {
      setIsConnecting(false);
    }
  };

  return (
    <div className="flex items-center gap-2">
      <Button
        variant="neutral"
        type="button"
        onClick={handleConnect}
        loading={isConnecting}
        icon={<PostHogLogo size={16} />}
      >
        {projects ? "Reconnect to PostHog" : "Connect with PostHog"}
      </Button>
      {projects && projects.length > 1 && (
        <Combobox<PostHogProject>
          label="PostHog project"
          labelHidden
          placeholder="Select a PostHog project"
          options={projects.map((p) => ({ label: p.name, value: p }))}
          selectedOption={selectedProject}
          setSelectedOption={(p) => {
            if (p) {
              setSelectedProject(p);
              onSelectProject(p);
              toast("success", `Loaded project "${p.name}" from PostHog`);
            }
          }}
        />
      )}
    </div>
  );
}
