import { Loading } from "@ui/Loading";
import {
  TableSchemaContainer,
  useSingleTableSchemaStatus,
} from "@common/features/data/components/TableSchema";
import { DataPanel } from "@common/features/data/components/DataPanel";
import { LearnMoreLink } from "./LearnMoreLink";

export function TableSchemaPanel({
  tableName,
  onClose,
}: {
  tableName: string;
  onClose: () => void;
}) {
  return (
    <DataPanel
      title={
        <>
          Schema for table{" "}
          <span className="font-mono text-[1.0625rem]">{tableName}</span>
        </>
      }
      onClose={onClose}
    >
      <SchemaBody tableName={tableName} />
    </DataPanel>
  );
}

function SchemaBody({ tableName }: { tableName: string }) {
  const tableSchemaStatus = useSingleTableSchemaStatus(tableName);
  if (tableSchemaStatus === undefined) {
    return <Loading />;
  }
  return (
    <>
      <LearnMoreLink
        name="schemas"
        link="https://docs.convex.dev/database/schemas"
      />
      <div className="px-1 sm:px-3">
        <TableSchemaContainer tableName={tableName} />
      </div>
    </>
  );
}
