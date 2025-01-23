import { Dialog, Transition } from "@headlessui/react";
import { ClosePanelButton } from "dashboard-common";
import { GenericDocument } from "convex/server";
import isEqual from "lodash/isEqual";
import { Fragment, useState } from "react";
import { ValidatorJSON } from "convex/values";
import { JavascriptDocumentsForm } from "./JavascriptDocumentsForm";

export function EditFieldsPanel({
  allRowsSelected,
  numRowsSelected,
  onClose,
  onSave,
  validator,
  shouldSurfaceValidatorErrors,
}: {
  allRowsSelected: boolean;
  numRowsSelected: number;
  onClose: () => void;
  onSave(documents: GenericDocument): Promise<any>;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
}) {
  const [fields, setFields] = useState(() => [{}]);
  const isDirty = !isEqual(fields, [{}]);

  const closeWithConfirmation = () => {
    if (isDirty) {
      // eslint-disable-next-line no-alert
      const shouldClose = window.confirm(
        `You have unsaved changes.
Press "Cancel" to return to the editor, or "OK" to discard unsaved changes.`,
      );
      if (!shouldClose) {
        return;
      }
    }
    onClose();
  };

  return (
    <Transition.Root
      show
      as={Fragment}
      appear
      afterLeave={closeWithConfirmation}
    >
      <Dialog
        as="div"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={closeWithConfirmation}
        data-testid="editFieldsPanel"
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
            <Dialog.Overlay className="absolute inset-0 transition-opacity" />
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
                <div className="flex h-full max-h-full flex-col overflow-hidden bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="mb-1 px-4 pt-6 sm:px-6">
                    <div className="flex items-center justify-between gap-4">
                      <Dialog.Title as="h4">
                        Bulk edit{" "}
                        {documentsLabel(numRowsSelected, allRowsSelected)}
                      </Dialog.Title>
                      <ClosePanelButton onClose={closeWithConfirmation} />
                    </div>
                  </div>
                  <Dialog.Description
                    as="div"
                    className="mb-2 px-4 text-xs text-content-primary sm:px-6"
                  >
                    <p>You can:</p>
                    <p>• Add new fields</p>
                    <p>• Overwrite existing fields</p>
                    <p>
                      • Remove existing fields by setting them to{" "}
                      <code className="rounded bg-background-tertiary p-[2px] text-content-primary">
                        undefined
                      </code>
                    </p>
                  </Dialog.Description>
                  <JavascriptDocumentsForm
                    documents={fields}
                    setDocuments={setFields}
                    onSave={async (array) => {
                      await onSave(array[0]);
                      onClose();
                    }}
                    mode="patchDocuments"
                    isDirty={isDirty}
                    validator={validator}
                    shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
                  />
                </div>
              </div>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition.Root>
  );
}

function documentsLabel(numDocuments: number, allRowsSelected: boolean) {
  return allRowsSelected && numDocuments !== 1
    ? "all documents"
    : numDocuments > 1
      ? `${numDocuments} documents`
      : "document";
}
