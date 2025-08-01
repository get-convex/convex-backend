import { Loading } from "@ui/Loading";
import {
  TableSchemaContainer,
  useSingleTableSchemaStatus,
} from "@common/features/data/components/TableSchema";
import { DataPanel } from "@common/features/data/components/DataPanel";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import Link from "next/link";

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
