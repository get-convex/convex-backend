import { DownloadIcon, EnvelopeClosedIcon } from "@radix-ui/react-icons";
import { Button, Spinner, Callout, Sheet } from "dashboard-common";
import { useGetZipExport } from "hooks/deploymentApi";
import { Fragment } from "react";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { useCurrentDeployment } from "api/deployments";
import { CompletedExport } from "system-udfs/convex/_system/frontend/common";

function LatestSnapshot({
  existingExport,
}: {
  existingExport: CompletedExport;
}) {
  const getZipExport = useGetZipExport(existingExport.format);
  const format = existingExport.format?.format;
  if (format !== "zip") {
    // Old export formats have not been generated since 2023-12, so all such
    // exports have expired.
    throw new Error(`unexpected export with format ${format}`);
  }
  const deployment = useCurrentDeployment();
  // This should match the format in `get_zip_export` which is the actual name
  // used by the download.
  const filename = `snapshot_${
    deployment!.name
  }_${existingExport.start_ts.toString()}.${format}`;

  return (
    <div>
      <h4 className="mb-2">Latest Snapshot</h4>
      <p className="mb-4 text-sm">Download a snapshot of your tables.</p>
      <div className="max-w-2xl rounded-md border">
        <div className="rounded-t-md bg-background-primary px-4 py-2 text-sm sm:flex sm:justify-between sm:gap-4">
          <div className="truncate">
            Created{" "}
            {new Date(
              Number(existingExport.start_ts / BigInt(1000000)),
            ).toLocaleString()}{" "}
          </div>
          <div className="truncate text-content-errorSecondary">
            Expires{" "}
            {new Date(
              Number(existingExport.expiration_ts / BigInt(1000000)),
            ).toLocaleString()}
          </div>
        </div>
        <div className="flex flex-col">
          <div className="flex w-full">
            <div className="flex w-full gap-x-4 overflow-hidden rounded-b-md md:grid-cols-[minmax(auto,calc(100%-10rem))_10rem]">
              <Fragment key={existingExport.zip_object_key}>
                <div className="flex items-center truncate whitespace-nowrap px-4 text-sm">
                  <div className="truncate">{filename}</div>
                </div>
                <div className="ml-auto flex items-center p-2 text-right text-sm font-medium">
                  <Button
                    size="sm"
                    variant="primary"
                    inline
                    aria-label="download"
                    href={getZipExport(existingExport._id)}
                  >
                    <DownloadIcon aria-label="Download" />
                    <span className="hidden md:flex">Download</span>
                  </Button>
                </div>
              </Fragment>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function SnapshotExport() {
  const existingExport = useQuery(udfs.latestExport.default);

  return (
    <Sheet>
      {existingExport ? (
        <div className="mt-4 text-content-primary">
          {["requested", "in_progress"].some(
            (s) => s === existingExport.state,
          ) ? (
            <div className="float-left flex items-center gap-2">
              <Spinner /> Export in progress
            </div>
          ) : existingExport.state === "failed" ? (
            <Callout variant="error">
              <div>
                Latest snapshot export failed. Please try again or contact us at{" "}
                <a
                  href="mailto:support@convex.dev"
                  className="items-center text-content-link dark:underline"
                >
                  <EnvelopeClosedIcon className="mr-0.5 inline" />
                  support@convex.dev
                </a>
              </div>
            </Callout>
          ) : existingExport.state === "completed" &&
            Date.now() <
              Number(existingExport.expiration_ts / BigInt(1000000)) ? (
            <div className="flex flex-col gap-4">
              <LatestSnapshot existingExport={existingExport} />
            </div>
          ) : (
            <span className="text-content-secondary">
              Latest snapshot export expired.
            </span>
          )}
        </div>
      ) : (
        <span className="mt-4 text-content-secondary">
          No snapshot export requested yet.
        </span>
      )}
    </Sheet>
  );
}
