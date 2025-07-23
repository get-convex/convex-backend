import { DataPanel } from "@common/features/data/components/DataPanel";
import { ConvexSchemaFilePath } from "@common/features/data/components/ConvexSchemaFilePath";
import { Loading } from "@ui/Loading";
import { useSingleTableSchemaStatus } from "./TableSchema";
import { IndexList } from "./IndexList";
import { LearnMoreLink } from "./LearnMoreLink";

export function TableIndexesPanel({
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
          Indexes for table{" "}
          <span className="font-mono text-[1.0625rem]">{tableName}</span>
        </>
      }
      onClose={onClose}
    >
      <IndexBody tableName={tableName} />
    </DataPanel>
  );
}

function IndexBody({ tableName }: { tableName: string }) {
  const tableSchemaStatus = useSingleTableSchemaStatus(tableName);
  if (tableSchemaStatus === undefined) {
    return <Loading />;
  }

  return (
    <>
      <LearnMoreLink
        name="indexes"
        link="https://docs.convex.dev/database/indexes"
      />
      <div className="px-4 sm:px-6">
        {tableSchemaStatus.isDefined ? (
          <IndexList tableName={tableName} />
        ) : (
          <>
            Once you add your table to your <ConvexSchemaFilePath /> file,
            you’ll be able to see any indexes you’ve defined here.
          </>
        )}
      </div>
    </>
  );
}
