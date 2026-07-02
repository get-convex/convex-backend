import { useMemo, useState } from "react";
import { CopyIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Modal } from "@ui/Modal";
import { TextInput } from "@ui/TextInput";
import { CopyButton } from "@common/elements/CopyButton";
import { validateConvexIdentifier } from "@common/features/data/lib/helpers";
import {
  generateColumnRenameScaffold,
  generateTableRenameScaffold,
} from "@common/features/data/lib/renameScaffold";
import { copyTextToClipboard, toast } from "@common/lib/utils";

type RenameDialogProps = {
  mode: "column" | "table";
  tableName: string;
  currentName: string;
  onClose: () => void;
};

export function RenameDialog({
  mode,
  tableName,
  currentName,
  onClose,
}: RenameDialogProps) {
  const [newName, setNewName] = useState("");
  const [backfill, setBackfill] = useState(true);
  const label = mode === "column" ? "Column name" : "Table name";
  const validationError =
    validateConvexIdentifier(newName, label) ??
    (newName === currentName ? `${label} must be different.` : undefined);
  const scaffold = useMemo(
    () =>
      mode === "column"
        ? generateColumnRenameScaffold({
            tableName,
            oldColumnName: currentName,
            newColumnName: newName || "newName",
            backfill,
          })
        : generateTableRenameScaffold({
            oldTableName: currentName,
            newTableName: newName || "newName",
            backfill,
          }),
    [backfill, currentName, mode, newName, tableName],
  );
  const preview = [scaffold.mutationCode, scaffold.schemaDiff]
    .filter(Boolean)
    .join("\n\n");
  const title = mode === "column" ? "Rename column" : "Rename table";

  const copyPreview = async () => {
    await copyTextToClipboard(preview);
    toast("success", "Rename scaffold copied to clipboard.", undefined, 2000);
  };

  return (
    <Modal title={title} onClose={onClose} size="md">
      <div className="flex flex-col gap-4 pb-3">
        <TextInput
          id="rename-new-name"
          label={`New ${mode} name`}
          autoFocus
          placeholder={currentName}
          value={newName}
          onChange={(event) => setNewName(event.target.value)}
          error={newName ? validationError : undefined}
        />

        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={backfill}
            onChange={() => setBackfill((value) => !value)}
            id="rename-backfill"
          />
          Backfill existing data
        </label>

        <div className="text-sm text-content-secondary">
          This does not apply a migration. Apply by adding the mutation to your{" "}
          <code>convex/</code> folder and updating <code>convex/schema.ts</code>{" "}
          as shown, then run <code>npx convex dev</code>.
          {mode === "column" &&
            " Schema diff shows the rename only; preserve your existing validator type for the column."}
        </div>

        {mode === "table" && (
          <Callout variant="error">
            Renaming a table assigns new document IDs. Any v.id() references to
            this table from other tables will need to be updated in the same
            migration.
          </Callout>
        )}

        <ScaffoldPreview
          title="Mutation"
          text={scaffold.mutationCode}
          emptyText="Backfill is off, so no mutation is generated."
        />
        <ScaffoldPreview title="Schema diff" text={scaffold.schemaDiff} />
      </div>

      <div className="flex w-full flex-wrap gap-2">
        <div className="grow" />
        <Button variant="neutral" onClick={onClose}>
          Close
        </Button>
        <Button
          disabled={!!validationError}
          onClick={() => void copyPreview()}
          icon={<CopyIcon />}
        >
          Copy mutation + schema diff to clipboard
        </Button>
      </div>
    </Modal>
  );
}

function ScaffoldPreview({
  title,
  text,
  emptyText,
}: {
  title: string;
  text: string;
  emptyText?: string;
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <h5>{title}</h5>
        <CopyButton text={text} disabled={!text} tip={`Copy ${title}`} />
      </div>
      <pre className="max-h-60 overflow-auto rounded-sm border bg-background-tertiary p-3 text-xs text-content-primary">
        {text || emptyText}
      </pre>
    </div>
  );
}
