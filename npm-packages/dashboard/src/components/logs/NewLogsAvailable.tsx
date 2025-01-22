import { ArrowDownIcon } from "@radix-ui/react-icons";
import { Button } from "dashboard-common";

export function NewLogsAvailable({ onClick }: { onClick(): void }) {
  return (
    <Button
      className="absolute bottom-2 right-8 z-20 motion-safe:animate-bounceIn"
      size="sm"
      type="button"
      // variant="neutral"
      onClick={onClick}
      icon={<ArrowDownIcon />}
    >
      New Logs
    </Button>
  );
}
