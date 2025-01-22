import { Value } from "convex/values";

import { ReadonlyCode, stringifyValue } from "dashboard-common";
import { DetailPanel } from "elements/DetailPanel";

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
        <div className="h-full rounded border p-4">
          <ReadonlyCode path="dataDetail" code={content} disableLineNumbers />
        </div>
      }
    />
  );
}
