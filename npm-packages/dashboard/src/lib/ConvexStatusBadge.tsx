import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import { ConvexStatus } from "./ConvexStatusWidget";

const STATUS_PAGE_URL = "https://status.convex.dev";

export function ConvexStatusBadge({ status }: { status: ConvexStatus }) {
  return (
    <a
      href={STATUS_PAGE_URL}
      target="_blank"
      rel="noreferrer"
      className={cn(
        "flex items-center gap-2 text-sm hover:underline",
        !status && "animate-fadeInFromLoading",
      )}
    >
      <span
        className={cn(
          "flex items-center gap-1 rounded-full px-2 py-1",
          status.indicator === "minor" &&
            "bg-background-warning text-content-warning",
          status.indicator === "major" &&
            "bg-background-error text-content-error",
          status.indicator === "critical" &&
            "bg-background-error text-content-error",
        )}
      >
        <span className="truncate">{status.description}</span>
        <ExternalLinkIcon className="min-w-4" />
      </span>
    </a>
  );
}
