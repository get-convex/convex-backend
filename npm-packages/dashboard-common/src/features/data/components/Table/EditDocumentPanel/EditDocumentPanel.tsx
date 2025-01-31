import { Fragment, useCallback, useMemo } from "react";
import { Dialog, Transition } from "@headlessui/react";
import { ValidatorJSON, Value } from "convex/values";
import { GenericDocument } from "convex/server";
import isEqual from "lodash/isEqual";
import omitBy from "lodash/omitBy";
import Link from "next/link";
import { createGlobalState } from "react-use";
import { JavascriptDocumentsForm } from "features/data/components/Table/EditDocumentPanel/JavascriptDocumentsForm";
import { ClosePanelButton } from "elements/ClosePanelButton";
import { useNents } from "lib/useNents";

export type EditDocumentPanelProps = {
  onClose: () => void;
  onSave(documents: GenericDocument[]): Promise<any>;
  defaultDocument: { [key: string]: Value };
  tableName: string;
  editing?: boolean;
  validator?: ValidatorJSON;
  shouldSurfaceValidatorErrors?: boolean;
};

type DocumentDraftKey = string;

export const useDocumentDrafts = createGlobalState<
  Record<DocumentDraftKey, GenericDocument[] | undefined>
>({});

// Even though we clear out drafts when editing documents,
// we still track their state separately so that they don't clear out
// the draft state of documents you were adding.
const getDocumentDraftKey = (
  componentId: string | null,
  tableName: string,
  editingId?: string,
) => `${editingId || "add"}-${componentId}-${tableName}`;

export function EditDocumentPanel({
  onClose,
  onSave,
  defaultDocument,
  tableName,
  editing = false,
  validator,
  shouldSurfaceValidatorErrors,
}: EditDocumentPanelProps) {
  const [drafts, setDrafts] = useDocumentDrafts();
  const defaultDocumentWithoutSystemFields = useMemo(
    () => omitBy(defaultDocument, (v, key) => key.startsWith("_")),
    [defaultDocument],
  );

  const componentId = useNents().selectedNent?.id ?? null;

  // Drafts are still used to keep track of state while the editor is open,
  // But they are cleared in edit mode when the editor is closed.
  // They are always cleared if the save button is clicked.
  const documents = drafts[
    getDocumentDraftKey(
      componentId,
      tableName,
      editing ? (defaultDocument._id as string) : undefined,
    )
  ] ?? [defaultDocumentWithoutSystemFields];
  const setDocuments = useCallback(
    (newDocuments?: GenericDocument[]) => {
      setDrafts((d) => ({
        ...d,
        [getDocumentDraftKey(
          componentId,
          tableName,
          editing ? (defaultDocument._id as string) : undefined,
        )]: newDocuments,
      }));
    },
    [componentId, defaultDocument._id, editing, setDrafts, tableName],
  );

  const saveAndClearDraft = async () => {
    await onSave(documents);
    onClose();
    setDocuments(undefined);
  };

  const isDirty = !isEqual(documents, [defaultDocumentWithoutSystemFields]);

  const docsSection = editing ? "editing-a-document" : "creating-documents";

  const closeAndMaybeClearDraft = () => {
    // We only need to confirm closing the dialog if the user is editing a document. When the user adds documents, we store them as a draft.
    if (editing) {
      if (isDirty) {
        // eslint-disable-next-line no-alert
        const shouldClose = window.confirm(
          `You have unsaved changes.
  Press "Cancel" to return to the editor, or "OK" to discard unsaved changes.`,
        );
        if (!shouldClose) {
          return;
        }
        // If the user is editing a document, clear the drafts. It can be a bit confusing to see a dirty state when opening the edit document dialog
        // again. Also, using the single-cell editor is more convenient than this editor anyway.
        setDocuments(undefined);
      }
    }
    onClose();
  };

  return (
    <Transition.Root
      show
      as={Fragment}
      appear
      afterLeave={closeAndMaybeClearDraft}
    >
      <Dialog
        as="div"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={closeAndMaybeClearDraft}
        data-testid="editDocumentPanel"
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
                <div className="flex h-full max-h-full flex-col overflow-hidden bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="mb-1 px-4 pt-6 sm:px-6">
                    <div className="flex items-center justify-between gap-4">
                      <Dialog.Title as="h4">
                        {editing ? (
                          <div className="flex items-center gap-2">
                            Edit Document{" "}
                            <code className="text-xs" aria-label="Document ID">
                              {defaultDocument._id as undefined | string}
                            </code>
                          </div>
                        ) : (
                          "Add new documents"
                        )}
                      </Dialog.Title>
                      <ClosePanelButton onClose={closeAndMaybeClearDraft} />
                    </div>
                  </div>
                  <Dialog.Description className="mb-2 px-4 text-xs text-content-primary sm:px-6">
                    <Link
                      passHref
                      href={`https://docs.convex.dev/dashboard/deployments/data#${docsSection}`}
                      className="text-content-link dark:underline"
                      target="_blank"
                    >
                      Learn more
                    </Link>{" "}
                    about editing documents.
                  </Dialog.Description>
                  <JavascriptDocumentsForm
                    validator={validator}
                    shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
                    documents={documents}
                    setDocuments={setDocuments}
                    onSave={saveAndClearDraft}
                    isDirty={isDirty}
                    mode={editing ? "editDocument" : "addDocuments"}
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
