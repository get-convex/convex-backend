import {
  TableSchemaContainer,
  useSingleTableSchemaStatus,
} from "components/dataBrowser/TableSchema";
import { Loading, ClosePanelButton } from "dashboard-common";
import { IndexList } from "components/dataBrowser/IndexList";
import { Fragment } from "react";
import { Transition, Dialog } from "@headlessui/react";
import Link from "next/link";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { ConvexSchemaFilePath } from "./ConvexSchemaFilePath";

export function TableSchemaAndIndexes({
  tableName,
  onClose,
}: {
  tableName: string;
  onClose: () => void;
}) {
  return (
    <Transition.Root show as={Fragment} appear afterLeave={onClose}>
      <Dialog
        as="div"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={onClose}
      >
        <div className="absolute inset-0 overflow-hidden">
          <Transition.Child
            as={Fragment}
            enter="ease-in-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in-out duration-300"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Dialog.Overlay className="absolute inset-0" />
          </Transition.Child>

          <div className="fixed inset-y-0 right-0 flex max-w-full pl-10">
            <Transition.Child
              as={Fragment}
              enter="transform transition ease-in-out duration-200 sm:duration-300"
              enterFrom="translate-x-full"
              enterTo="translate-x-0"
              leave="transform transition ease-in-out duration-200 sm:duration-300"
              leaveFrom="translate-x-0"
              leaveTo="translate-x-full"
            >
              <div className="w-screen max-w-2xl">
                <div className="flex h-full max-h-full flex-col overflow-y-auto bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="mb-1 px-4 pt-6 sm:px-6">
                    <div className="flex items-center justify-between gap-4">
                      <Dialog.Title as="h4">
                        Schema for table{" "}
                        <span className="font-mono text-[1.0625rem]">
                          {tableName}
                        </span>
                      </Dialog.Title>
                      <ClosePanelButton onClose={onClose} />
                    </div>
                  </div>
                  <SchemaAndIndexBody tableName={tableName} />
                </div>
              </div>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition.Root>
  );
}

function SchemaAndIndexBody({ tableName }: { tableName: string }) {
  const tableSchemaStatus = useSingleTableSchemaStatus(tableName);
  if (
    tableSchemaStatus === undefined ||
    tableSchemaStatus.isValidationRunning
  ) {
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
        className="inline-flex items-center text-content-link dark:underline"
        target="_blank"
      >
        {name}
        <ExternalLinkIcon className="ml-0.5 h-3 w-3" />
      </Link>
    </div>
  );
}
