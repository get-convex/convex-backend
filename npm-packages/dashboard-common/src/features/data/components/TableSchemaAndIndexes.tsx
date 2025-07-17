import Link from "next/link";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Loading } from "@ui/Loading";
import { IndexList } from "@common/features/data/components/IndexList";
import {
  TableSchemaContainer,
  useSingleTableSchemaStatus,
} from "@common/features/data/components/TableSchema";
import { ConvexSchemaFilePath } from "@common/features/data/components/ConvexSchemaFilePath";
import { DataPanel } from "@common/features/data/components/DataPanel";

export function TableSchemaAndIndexes({
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
      <SchemaAndIndexBody tableName={tableName} />
    </DataPanel>
  );
}

function SchemaAndIndexBody({ tableName }: { tableName: string }) {
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
      <div className="mb-1 px-4 pt-6 font-semibold text-content-primary sm:px-6">
        Indexes
      </div>
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
            you'll be able to see any indexes you've defined here.
          </>
        )}
      </div>
    </>
  );
}

function LearnMoreLink({ name, link }: { name: string; link: string }) {
  return (
    <div className="mb-2 px-4 text-xs text-content-primary sm:px-6">
      Learn more about{" "}
      <Link
        passHref
        href={link}
        className="inline-flex items-center text-content-link"
        target="_blank"
      >
        {name}
        <ExternalLinkIcon className="ml-0.5 h-3 w-3" />
      </Link>
    </div>
  );
}
