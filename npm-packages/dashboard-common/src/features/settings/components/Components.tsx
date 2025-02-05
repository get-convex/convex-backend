import React, { useState } from "react";
import Link from "next/link";
import { TrashIcon } from "@radix-ui/react-icons";
import { useDeleteComponent } from "@common/features/settings/lib/api";
import { Sheet } from "@common/elements/Sheet";
import { Nent } from "@common/lib/useNents";
import { ConfirmationDialog } from "@common/elements/ConfirmationDialog";
import { Button } from "@common/elements/Button";

export function Components({ nents }: { nents: Nent[] }) {
  const sortedNents = [...nents]
    .filter((nent) => nent.name !== null)
    .sort((a, b) => {
      if (a.state === "active" && b.state !== "active") {
        return 1;
      }
      if (a.state !== "active" && b.state === "active") {
        return -1;
      }
      return 0;
    });

  return (
    <Sheet className="h-fit">
      <div>
        <h3 className="mb-4">Components</h3>
        <p className="flex flex-wrap gap-1">
          This page lists all of the components that are configured in your
          project's <pre>convex.config.ts</pre> file. You may delete components
          that have been unmounted.{" "}
          <Link
            // TODO(ENG-7301): Update this link to the correct URL
            href="https://docs.convex.dev/components"
            className="text-content-link hover:underline"
          >
            Learn more
          </Link>
        </p>
        {sortedNents.length === 0 ? (
          <div className="my-4 text-content-secondary">
            There are no components installed in this deployment.
          </div>
        ) : (
          <div className="my-4 flex flex-col divide-y">
            {sortedNents.map((nent, i) => (
              <ComponentListItem key={i} nent={nent} />
            ))}
          </div>
        )}
      </div>
    </Sheet>
  );
}

function ComponentListItem({ nent }: { nent: Nent }) {
  const [showConfirmation, setShowConfirmation] = useState(false);
  const deleteComponent = useDeleteComponent();
  return (
    <div
      className="grid w-full items-center gap-4 py-2"
      style={{
        gridTemplateColumns: "1fr auto",
      }}
    >
      {showConfirmation && (
        <ConfirmationDialog
          onConfirm={async () => {
            // You can only delete non-root components
            nent.id && (await deleteComponent(nent.id));
            setShowConfirmation(false);
          }}
          onClose={() => setShowConfirmation(false)}
          validationText={nent.path}
          confirmText="Delete"
          dialogTitle="Delete Component"
          dialogBody="Deleting this component will destroy all of its functions and data. It will also delete all subcomponents of this component."
        />
      )}
      <span className="flex items-center truncate">
        {nent.path}
        {nent.state !== "active" && (
          <span className="ml-1 text-content-secondary">(unmounted)</span>
        )}
      </span>
      <Button
        size="sm"
        inline
        className="ml-auto"
        tip={
          nent.state === "active" &&
          "You must unmount your component before it can be deleted."
        }
        tipSide="left"
        onClick={() => {
          setShowConfirmation(true);
        }}
        variant="danger"
        icon={<TrashIcon />}
        disabled={nent.state === "active"}
      >
        Delete
      </Button>
    </div>
  );
}
