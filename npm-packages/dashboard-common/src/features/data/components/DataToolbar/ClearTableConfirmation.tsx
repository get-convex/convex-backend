import { captureException } from "@sentry/nextjs";
import { Cursor } from "convex/server";
import { toast } from "dashboard-common";
import { useState } from "react";
import { useMountedState } from "react-use";
import { useInvalidateShapes } from "../../lib/api";
import { ConfirmationDialog } from "../../../../elements/ConfirmationDialog";

export function ClearTableConfirmation({
  numRows,
  closePopup,
  tableName,
  isProd,
  clearTable,
  clearSelectedRows,
}: {
  numRows?: number;
  closePopup: () => void;
  clearTable: (cursor: Cursor | null) => Promise<{
    continueCursor: Cursor;
    deleted: number;
    hasMore: boolean;
  }>;
  clearSelectedRows: () => void;
  tableName: string;
  isProd: boolean;
}) {
  const [initialNumRows, setInitialNumRows] = useState(numRows);
  const [numDeleted, setNumDeleted] = useState(0);
  const [isClearing, setIsClearing] = useState(false);
  const progressPct = initialNumRows
    ? Math.min(100, Math.floor((numDeleted / initialNumRows) * 100))
    : 0;

  const closeWithConfirmation = () => {
    if (isClearing) {
      // eslint-disable-next-line no-alert
      const shouldClose = window.confirm(
        "Closing the popup will cancel the clear table operation with the table partially cleared. Are you sure you want to continue?",
      );
      if (!shouldClose) {
        return;
      }
    }
    closePopup();
  };

  const isMounted = useMountedState();

  const invalidateShapes = useInvalidateShapes();

  return (
    <ConfirmationDialog
      onClose={closeWithConfirmation}
      onConfirm={async () => {
        setInitialNumRows(numRows);
        setIsClearing(true);
        let nextCursor: Cursor | null = null;
        let hasMoreDocuments = true;
        while (isMounted() && hasMoreDocuments) {
          try {
            const {
              hasMore,
              deleted,
              continueCursor,
            }: Awaited<ReturnType<typeof clearTable>> =
              await clearTable(nextCursor);
            hasMoreDocuments = hasMore;
            nextCursor = continueCursor;
            setNumDeleted((prev) => prev + deleted);
          } catch (e) {
            hasMoreDocuments = false;
            captureException(e);
            toast(
              "error",
              "Failed to clear table. Please try again or contact support.",
            );
            setIsClearing(false);
            await invalidateShapes();
            return;
          }
        }
        setIsClearing(false);
        clearSelectedRows();

        await invalidateShapes();
        if (isMounted()) {
          toast("success", "Table cleared.");
        } else {
          toast("info", "Table partially cleared.");
        }
      }}
      validationText={isProd ? tableName : undefined}
      confirmText="Confirm"
      variant="danger"
      dialogTitle="Clear table"
      disableCancel={isClearing}
      dialogBody={
        isClearing ? (
          <div className="flex flex-col gap-2">
            <span className="text-xs font-semibold">{progressPct}% done</span>
            <div className="mb-4 h-2.5 w-full rounded-full bg-background-tertiary">
              <div
                className="h-2.5 animate-pulse rounded-l-full bg-util-accent transition-[width]"
                style={{ width: `${progressPct}%` }}
              />
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <div>Delete all documents in this table?</div>
            <div>
              Clearing a large table may take a few minutes to complete. Keep
              this dialogue open while clearing is in progress. Documents that
              are created during the clear operation may not be deleted.
            </div>
          </div>
        )
      }
    />
  );
}
