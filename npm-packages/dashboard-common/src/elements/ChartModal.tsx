import React from "react";
import { Modal } from "@ui/Modal";
import { BigChart } from "@common/elements/BigChart";
import { ChartDataSource } from "@common/lib/charts/types";

export function ChartModal({
  onClose,
  dataSources,
  chartTitle,
  entityName,
  labels,
}: {
  onClose: () => void;
  dataSources: ChartDataSource[];
  labels: string[];
  chartTitle: string;
  entityName: string;
}) {
  return (
    <Modal
      size="lg"
      onClose={onClose}
      title={
        <div className="flex items-center gap-1">
          <pre className="inline rounded-md border bg-background-tertiary p-1 text-xs text-content-primary">
            {entityName}
          </pre>
          {chartTitle}
        </div>
      }
    >
      <BigChart dataSources={dataSources} syncId="ChartModal" labels={labels} />
    </Modal>
  );
}
