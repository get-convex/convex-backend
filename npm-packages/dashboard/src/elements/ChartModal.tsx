import React from "react";
import { ChartDataSource, BigChart } from "dashboard-common";
import { Modal } from "./Modal";

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
