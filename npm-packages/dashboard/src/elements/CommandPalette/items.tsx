import { Command, useCommandState } from "cmdk";
import React, { useContext } from "react";
import { CaretRightIcon, Pencil2Icon, StackIcon } from "@radix-ui/react-icons";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { Button } from "@ui/Button";
import { cn } from "@ui/cn";
import type { DeploymentType } from "@convex-dev/platform/managementApi";
import {
  deploymentTypeColorClasses,
  deploymentTypeLabel,
} from "@common/lib/deploymentTypeColorClasses";
import { useProjectById } from "api/projects";
import type { PlatformDeploymentResponse, ProjectDetails } from "generatedApi";
import type { NavigationTarget } from "./navigation";
import { REMOTE_VALUE_PREFIX } from "./navigation";

// Items whose default action is direct navigation drill into their nested
// view instead when this flag is set. The dialog sets it from Shift+Enter and
// ArrowRight just before cmdk fires the selection, and clears it right after.
export const DrillModifierContext = React.createContext<{ current: boolean }>({
  current: false,
});

export function useConsumeDrillModifier() {
  const flag = useContext(DrillModifierContext);
  return () => {
    const active = flag.current;
    flag.current = false;
    return active;
  };
}

// Indices of `text` covered by an occurrence of any whitespace-separated
// token of `query`, case-insensitively. Mirrors the palette filter's
// substring-token matching.
function matchedIndices(query: string, text: string): Set<number> {
  const indices = new Set<number>();
  const lowerText = text.toLowerCase();
  for (const token of query.trim().toLowerCase().split(/\s+/)) {
    if (!token) {
      continue;
    }
    let idx = lowerText.indexOf(token);
    while (idx !== -1) {
      for (let i = idx; i < idx + token.length; i++) {
        indices.add(i);
      }
      idx = lowerText.indexOf(token, idx + 1);
    }
  }
  return indices;
}

// Renders text with the characters matching the current palette search
// emphasized, in the same style as the schema page's search.
export function HighlightedText({ text }: { text: string }) {
  const search = useCommandState((state) => state.search);
  if (!search.trim()) {
    return <span>{text}</span>;
  }
  const indices = matchedIndices(search, text);
  if (indices.size === 0) {
    return <span>{text}</span>;
  }
  const parts: React.ReactNode[] = [];
  let buffer = "";
  let bufferHighlighted = false;
  const flush = () => {
    if (!buffer) {
      return;
    }
    parts.push(
      bufferHighlighted ? (
        <span key={parts.length} className="font-semibold text-content-accent">
          {buffer}
        </span>
      ) : (
        <span key={parts.length}>{buffer}</span>
      ),
    );
    buffer = "";
  };
  for (let i = 0; i < text.length; i++) {
    const highlighted = indices.has(i);
    if (highlighted !== bufferHighlighted) {
      flush();
      bufferHighlighted = highlighted;
    }
    buffer += text[i];
  }
  flush();
  // Wrap the runs in a single element: many items lay their children out with
  // flex + gap, which would otherwise insert space between the runs.
  return <span>{parts}</span>;
}

export function NavigationItem({
  target,
  onNavigate,
  hint,
}: {
  target: NavigationTarget;
  onNavigate: (href: string) => void;
  // Right-aligned annotation, e.g. "Current Page".
  hint?: string;
}) {
  const { label, href, Icon, parent, keywords } = target;
  const searchKeywords = keywords ?? [label];
  return (
    <Command.Item
      // Section targets share their page's href, so include the label to
      // keep values unique.
      value={`nav:${href}:${label}`}
      keywords={parent ? [...searchKeywords, parent] : searchKeywords}
      onSelect={() => onNavigate(href)}
    >
      <Icon className="text-content-secondary" />
      {/* Two lines: the page/section itself, then where it lives. */}
      <span className="flex min-w-0 flex-col">
        <span className="truncate">
          <HighlightedText text={label} />
        </span>
        {parent && (
          <span className="truncate text-xs text-content-tertiary">
            <HighlightedText text={parent} />
          </span>
        )}
      </span>
      {hint && (
        <span className="ml-auto shrink-0 text-xs text-content-tertiary">
          {hint}
        </span>
      )}
    </Command.Item>
  );
}

// A non-navigation command (drill-in page, theme change, tool, …).
export function ActionItem({
  value,
  onSelect,
  Icon,
  label,
  drillIn = false,
}: {
  value: string;
  onSelect: () => void;
  Icon: React.FC<{ className?: string }>;
  label: string;
  drillIn?: boolean;
}) {
  return (
    <Command.Item value={value} keywords={[label]} onSelect={onSelect}>
      <Icon className="text-content-secondary" />
      <HighlightedText text={label} />
      {drillIn && <DrillInHint />}
    </Command.Item>
  );
}

// Reports that palette content is loading. Renders nothing: the dialog shows
// the spinner in the search input while any signal is mounted, instead of a
// loading row inside the list.
export const PaletteLoadingContext = React.createContext<
  (() => () => void) | null
>(null);

export function LoadingSignal() {
  const beginLoading = React.useContext(PaletteLoadingContext);
  React.useEffect(() => beginLoading?.(), [beginLoading]);
  return null;
}

export function DrillInHint({
  kind,
  onDrill,
}: {
  kind?: string;
  // When set, the caret becomes a click target for drilling into the item's
  // nested view (the row itself navigates directly).
  onDrill?: () => void;
}) {
  return (
    <span className="ml-auto flex shrink-0 items-center gap-1 text-xs text-content-tertiary">
      {kind}
      {onDrill ? (
        <Button
          variant="unstyled"
          aria-label="Browse"
          tip="Browse (⇧⏎)"
          className="rounded-sm p-0.5 hover:bg-background-primary"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onDrill();
          }}
        >
          <CaretRightIcon className="size-4" />
        </Button>
      ) : (
        <CaretRightIcon className="size-4" />
      )}
    </span>
  );
}

// A project result. Selecting it goes straight to the project; the drill
// modifier (Shift+Enter / ArrowRight / clicking the caret) opens its pages
// and deployments instead.
export function ProjectItem({
  project,
  teamSlug,
  teamName,
  onNavigate,
  onDrill,
}: {
  project: ProjectDetails;
  teamSlug: string;
  // The parent team, for the item's second line.
  teamName?: string;
  onNavigate: (href: string) => void;
  onDrill: () => void;
}) {
  const consumeDrillModifier = useConsumeDrillModifier();
  return (
    <Command.Item
      value={`${REMOTE_VALUE_PREFIX}project:${project.id}`}
      className="animate-fadeInFromLoading"
      onSelect={() =>
        consumeDrillModifier()
          ? onDrill()
          : onNavigate(`/t/${teamSlug}/${project.slug}`)
      }
    >
      <StackIcon className="text-content-secondary" />
      {/* Two lines: the project, then the team it belongs to. */}
      <span className="flex min-w-0 flex-col">
        <span className="flex min-w-0 items-baseline gap-1.5">
          <span className="truncate">
            <HighlightedText text={project.name || project.slug} />
          </span>
          <span className="truncate text-xs text-content-tertiary">
            <HighlightedText text={project.slug} />
          </span>
        </span>
        <span className="truncate text-xs text-content-tertiary">
          {teamName ?? teamSlug}
        </span>
      </span>
      <DrillInHint kind="Project" onDrill={onDrill} />
    </Command.Item>
  );
}

// A deployment result. Selecting it goes straight to the deployment's Health
// page; the drill modifier opens its page list instead (also the fallback
// while the project slug needed for the direct link is still loading).
export function DeploymentItem({
  deployment,
  teamSlug,
  projectSlug: knownProjectSlug,
  onNavigate,
  onDrill,
  remote = false,
  showProject = false,
}: {
  deployment: PlatformDeploymentResponse;
  teamSlug: string;
  projectSlug?: string;
  onNavigate: (href: string) => void;
  onDrill: () => void;
  // Whether this item comes from server-side search (bypasses the client
  // filter) rather than an already-loaded local list.
  remote?: boolean;
  // Show the deployment's project, for lists that span multiple projects.
  showProject?: boolean;
}) {
  const consumeDrillModifier = useConsumeDrillModifier();
  const { project } = useProjectById(deployment.projectId);
  const projectSlug = knownProjectSlug ?? project?.slug;
  const typeLabel = deploymentTypeLabel(deployment.deploymentType);
  const primary =
    deployment.kind === "cloud" ? deployment.reference : deployment.name;
  return (
    <Command.Item
      value={`${remote ? REMOTE_VALUE_PREFIX : ""}deployment:${deployment.name}`}
      className="animate-fadeInFromLoading"
      keywords={remote ? undefined : [primary, deployment.name, typeLabel]}
      onSelect={() =>
        consumeDrillModifier() || !projectSlug
          ? onDrill()
          : onNavigate(`/t/${teamSlug}/${projectSlug}/${deployment.name}`)
      }
    >
      <div
        className={cn(
          "inline-flex shrink-0 items-center justify-center rounded-full p-1",
          deploymentTypeColorClasses(deployment.deploymentType),
        )}
      >
        <DeploymentTypeIcon deploymentType={deployment.deploymentType} />
      </div>
      {/* Two lines: the deployment, then the project it belongs to. */}
      <span className="flex min-w-0 flex-col">
        {/* Baseline-align the differently-sized reference and name. */}
        <span className="flex min-w-0 items-baseline gap-1.5">
          <span className="truncate">
            <HighlightedText text={primary} />
          </span>
          <span className="truncate text-xs text-content-tertiary">
            <HighlightedText text={deployment.name} />
          </span>
        </span>
        {showProject && project && (
          <span className="animate-fadeInFromLoading truncate text-xs text-content-tertiary">
            <HighlightedText text={project.name || project.slug} />
          </span>
        )}
      </span>
      <DrillInHint kind={typeLabel} onDrill={onDrill} />
    </Command.Item>
  );
}

function DeploymentTypeIcon({
  deploymentType,
}: {
  deploymentType: DeploymentType;
}) {
  switch (deploymentType) {
    case "prod":
      return <SignalIcon className="size-3.5" />;
    case "dev":
      return <CommandLineIcon className="size-3.5" />;
    case "preview":
      return <Pencil2Icon className="size-3.5" />;
    case "custom":
      return <WrenchIcon className="size-3.5" />;
    default: {
      deploymentType satisfies never;
      return null;
    }
  }
}
