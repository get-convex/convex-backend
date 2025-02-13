import Link from "next/link";
import { Modal } from "@common/elements/Modal";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { SourceMissingPanel } from "@common/elements/SourceMissingPanel";

type Props = {
  onClose: () => void;
  contents: string;
  displayName: string;
};

export function FileModal({ contents, onClose, displayName }: Props) {
  return (
    <Modal
      title={
        <div className="flex items-center gap-3">
          Cron Jobs
          <pre className="inline rounded border bg-background-tertiary p-1 text-xs text-content-primary">
            {displayName}
          </pre>
        </div>
      }
      description={
        <div className="max-w-[32rem]">
          Cron jobs are defined in this file.{" "}
          <Link
            href="https://docs.convex.dev/scheduling/cron-jobs"
            passHref
            className="text-content-link"
            target="_blank"
          >
            Learn more
          </Link>
          .
        </div>
      }
      onClose={onClose}
    >
      <div className="rounded border p-4" style={{ height: "80vh" }}>
        {contents ? (
          <ReadonlyCode
            path={displayName}
            code={contents.trimEnd()}
            language="javascript"
          />
        ) : (
          <SourceMissingPanel />
        )}
      </div>
    </Modal>
  );
}
