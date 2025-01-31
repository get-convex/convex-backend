import { CrossCircledIcon } from "@radix-ui/react-icons";
import { LogOutcome } from "lib/useLogs";

export function LogStatusLine({ outcome }: { outcome: LogOutcome }) {
  return (
    <p className="flex items-center gap-1">
      {(outcome.status === "failure" ||
        (outcome.statusCode && Number(outcome.statusCode) >= 400)) && (
        <CrossCircledIcon />
      )}
      {outcome.statusCode !== null ? outcome.statusCode : outcome.status}
    </p>
  );
}
