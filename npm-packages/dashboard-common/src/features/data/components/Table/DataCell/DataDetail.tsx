import { Value } from "convex/values";

import { DetailPanel } from "@common/elements/DetailPanel";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { stringifyValue } from "@common/lib/stringifyValue";

export function DataDetail({
  value,
  header,
  onClose,
}: {
  value: Value;
  header: React.ReactNode;
  onClose: () => void;
}) {
  // Only stringify non-string values.
  const content = stringifyValue(value, true);
  return (
    <DetailPanel
      onClose={onClose}
      header={header}
      content={
        <div className="h-full rounded-sm border p-4">
          <ReadonlyCode path="dataDetail" code={content} disableLineNumbers />
        </div>
      }
    />
  );
}
