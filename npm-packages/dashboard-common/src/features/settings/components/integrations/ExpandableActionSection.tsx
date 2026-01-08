import { useState } from "react";
import { Button } from "@ui/Button";

export interface ExpandableActionConfig {
  trigger: {
    label: string;
    className?: string;
  };
  expanded: {
    title: string | React.ReactNode;
    description: string | React.ReactNode;
    variant?: "danger" | "warning" | "neutral";
    actions: {
      primary: {
        label: string;
        onClick: () => void | Promise<void>;
        variant?: "primary" | "danger" | "neutral";
        disabled?: boolean;
        tip?: string;
      };
      secondary?: {
        label: string;
        onClick: () => void | Promise<void>;
        variant?: "primary" | "danger" | "neutral";
        disabled?: boolean;
      };
    };
  };
}

export function ExpandableActionSection({
  config,
  isLoading = false,
  children,
}: {
  config: ExpandableActionConfig;
  isLoading?: boolean;
  children?: React.ReactNode;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  const handlePrimaryAction = async () => {
    await config.expanded.actions.primary.onClick();
    setIsExpanded(false);
  };

  const handleSecondaryAction = async () => {
    if (config.expanded.actions.secondary) {
      await config.expanded.actions.secondary.onClick();
      setIsExpanded(false);
    }
  };

  const handleCancel = () => {
    setIsExpanded(false);
  };

  const borderColorClass =
    config.expanded.variant === "danger"
      ? "border-content-error"
      : config.expanded.variant === "warning"
        ? "border-content-warning"
        : "border";

  const titleColorClass =
    config.expanded.variant === "danger"
      ? "text-content-error"
      : config.expanded.variant === "warning"
        ? "text-content-warning"
        : "text-content-primary";

  if (!isExpanded) {
    return (
      <div>
        <Button variant="neutral" size="sm" onClick={() => setIsExpanded(true)}>
          {config.trigger.label}
        </Button>
      </div>
    );
  }

  return (
    <div
      className={`flex flex-col gap-2 rounded-sm border ${borderColorClass} p-3`}
    >
      <div className="flex flex-col gap-1">
        <div className={`text-sm font-semibold ${titleColorClass}`}>
          {config.expanded.title}
        </div>
        <div className="text-xs text-content-secondary">
          {config.expanded.description}
        </div>
      </div>
      {children}
      <div className="flex gap-2">
        <Button
          variant={config.expanded.actions.primary.variant || "primary"}
          size="sm"
          onClick={handlePrimaryAction}
          disabled={config.expanded.actions.primary.disabled}
          loading={isLoading}
          tip={config.expanded.actions.primary.tip}
        >
          {config.expanded.actions.primary.label}
        </Button>
        {config.expanded.actions.secondary && (
          <Button
            variant={config.expanded.actions.secondary.variant || "neutral"}
            size="sm"
            onClick={handleSecondaryAction}
            disabled={config.expanded.actions.secondary.disabled}
            loading={isLoading}
          >
            {config.expanded.actions.secondary.label}
          </Button>
        )}
        <Button
          variant="neutral"
          size="sm"
          onClick={handleCancel}
          disabled={isLoading}
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}
