import { useContext, useState } from "react";
import { DeploymentInfoContext } from "dashboard-common/lib/deploymentContext";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  Cross2Icon,
  ExternalLinkIcon,
} from "@radix-ui/react-icons";
import { Button } from "dashboard-common/elements/Button";
import { CopyTextButton } from "dashboard-common/elements/CopyTextButton";
import Link from "next/link";
import { cn } from "dashboard-common/lib/cn";

// Little toast to prompt users who are trying out Convex before creating
// an account about the Convex cloud product.
export function ConvexCloudReminderToast() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isTryItOutDeployment = deployment?.name?.startsWith("tryitout-");
  const [isExpanded, setIsExpanded] = useState(false);
  const [isDismissed, setIsDismissed] = useState(false);

  if (!isTryItOutDeployment || isDismissed) {
    return null;
  }

  return (
    // Positioned in the bottom left corner, high enough to not block the
    // sidebar collapse button.
    <div className="absolute bottom-12 left-4 z-50">
      <div
        className="w-96 rounded-lg border border-purple-700 bg-background-secondary shadow-lg"
        role="region"
        aria-label="Convex cloud notice"
      >
        <div className="relative">
          <Button
            variant="unstyled"
            className={cn(
              "flex w-full cursor-pointer items-center justify-between rounded-lg p-2 text-sm font-medium text-purple-700 hover:bg-background-tertiary focus:outline-none focus:ring-2 focus:ring-purple-700",
              isExpanded && "border-b border-purple-500",
            )}
            onClick={() => setIsExpanded(!isExpanded)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                setIsExpanded(!isExpanded);
              }
            }}
            aria-expanded={isExpanded}
            aria-controls="tryitout-details"
          >
            <div className="flex items-center gap-2">
              {isExpanded ? (
                <ChevronDownIcon className="h-4 w-4" />
              ) : (
                <ChevronRightIcon className="h-4 w-4" />
              )}
              <span>Enjoying Convex? Ready to deploy your app?</span>
            </div>
            <Button
              variant="unstyled"
              className="rounded-full p-1 text-purple-700 hover:bg-purple-100"
              onClick={(e: React.MouseEvent) => {
                e.stopPropagation();
                setIsDismissed(true);
              }}
              aria-label="Dismiss"
            >
              <Cross2Icon className="h-4 w-4" />
            </Button>
          </Button>
        </div>
        {isExpanded && (
          <div
            id="tryitout-details"
            className="flex flex-col gap-2 border-purple-500 px-4 py-3 text-sm text-content-primary"
          >
            <p>You are currently trying out Convex by running it locally.</p>
            <p>
              If you're ready to deploy your app and share it with the world or
              want to access more features with the cloud product, create a
              Convex account and automatically link this deployment:
            </p>
            <p className="inline-flex items-center gap-2">
              Run this in your terminal:
              <CopyTextButton text="npx convex login" />
            </p>
            <Link
              href="https://docs.convex.dev"
              className="inline-flex items-center gap-2 text-content-link hover:underline"
              target="_blank"
            >
              Learn more about Convex
              <ExternalLinkIcon className="h-4 w-4" />
            </Link>
          </div>
        )}
      </div>
    </div>
  );
}
