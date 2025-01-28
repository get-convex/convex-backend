import { GenericId, Value } from "convex/values";
import { GenericDocument } from "convex/server";
import classNames from "classnames";
import React, { Fragment, memo, useRef, useState } from "react";
import { useClickAway, useHoverDirty } from "react-use";
import { areEqual } from "react-window";
import { usePopper } from "react-popper";
import { ColumnInstance } from "react-table";
import {
  CheckCircledIcon,
  DotsVerticalIcon,
  Link2Icon,
} from "@radix-ui/react-icons";
import { Portal, Transition } from "@headlessui/react";
import { useTableDensity } from "../../../lib/useTableDensity";

import { ProductionEditsConfirmationDialog } from "../../../../../elements/ProductionEditsConfirmationDialog";

import { KeyboardShortcut } from "../../../../../elements/KeyboardShortcut";
import { DataDetail } from "./DataDetail";
import { CellEditor } from "./CellEditor";
import { DataCellValue } from "./DataCellValue";

import type { usePatchDocumentField } from "../utils/usePatchDocumentField";
import { arrowKeyHandler } from "../utils/arrowKeyHandler";
import {
  OpenContextMenu,
  useActionHotkeys,
  useCellActions,
} from "./utils/cellActions";
import { usePasteListener } from "./utils/usePasteListener";
import { useTrackCellChanges } from "./utils/useTrackCellChanges";
import { useValidator } from "./utils/useValidator";
import { SchemaJson } from "../../../../../lib/format";
import { stringifyValue } from "../../../../../lib/stringifyValue";
import { Button } from "../../../../../elements/Button";

export type DataCellProps = {
  value: Value;
  document: GenericDocument;
  column: ColumnInstance<GenericDocument>;
  editDocument: () => void;
  areEditsAuthorized: boolean;
  onAuthorizeEdits?: () => void;
  rowId: GenericId<string>;
  didRowChange: boolean;
  width?: string;
  inferIsDate: boolean;
  patchDocument: ReturnType<typeof usePatchDocumentField>;
  tableName: string;
  onOpenContextMenu: OpenContextMenu;
  onCloseContextMenu: () => void;
  canManageTable: boolean;
  activeSchema: SchemaJson | null;
  isContextMenuOpen: boolean;
};

export const DataCell = memo(DataCellImpl, areEqual);

function DataCellImpl({
  value,
  column,
  onAuthorizeEdits,
  areEditsAuthorized,
  width,
  rowId,
  document,
  didRowChange,
  inferIsDate = false,
  patchDocument,
  tableName,
  onOpenContextMenu,
  onCloseContextMenu,
  canManageTable,
  activeSchema,
  editDocument,
  isContextMenuOpen,
}: DataCellProps) {
  const cellRef = useRef<HTMLDivElement | null>(null);
  const cellButtonRef = useRef<HTMLButtonElement>(null);

  // Derive all the information needed to render the cell
  const columnName = column.Header as string;
  const stringValue = typeof value === "string" ? value : stringifyValue(value);
  const isHoveringCell = useHoverDirty(cellRef);
  const isSystemField = columnName?.startsWith("_");
  const isEditable = !isSystemField && canManageTable;
  const isDateField = columnName === "_creationTime";

  // State for showing various modals and popovers
  const [showEnableProdEditsModal, setShowEnableProdEditsModal] =
    useState(false);
  const [showEditor, setShowEditor] = useState(false);
  const [pastedValue, setPastedValue] = useState<Value>();
  const [showDetail, setShowDetail] = useState(false);
  const [showDocumentDetail, setShowDocumentDetail] = useState(false);

  // Mega hook to generate all the actions
  // that can be performed on a cell
  // To keep the component code clean
  const {
    didJustCopy,
    idReferenceLink,
    copyValue,
    copyDocument,
    editValue,
    goToDoc,
    viewValue,
    viewDocument,
    contextMenuCallback,
  } = useCellActions({
    cellRef,
    onOpenContextMenu,
    onCloseContextMenu,
    columnName,
    rowId,
    value,
    document,
    areEditsAuthorized,
    onAuthorizeEdits,
    canManageTable,
    setPastedValue,
    setShowEditor,
    setShowEnableProdEditsModal,
    setShowDetail,
    setShowDocumentDetail,
    editDocument,
  });

  const hotkeyRefs = useActionHotkeys({
    copyCb: copyValue,
    copyDocCb: copyDocument,
    viewCb: viewValue,
    viewDocCb: viewDocument,
    editCb: editValue,
    editDocCb: editDocument,
    goToDocCb: goToDoc,
    openContextMenu: () => {
      cellRef.current &&
        contextMenuCallback({
          x: cellRef.current!.getBoundingClientRect().right,
          y: cellRef.current!.getBoundingClientRect().top,
        });
    },
  });

  const { shouldSurfaceValidatorErrors, allowTopLevelUndefined, validator } =
    useValidator(activeSchema, tableName, columnName);

  usePasteListener(cellRef, columnName, editValue, allowTopLevelUndefined);

  const didValueJustChange = useTrackCellChanges({
    value,
    didRowChange,
  });

  const { densityValues } = useTableDensity();

  // Controls the copied value popper that shows up when a value is copied
  const [copiedPopperElement, setCopiedPopperElement] =
    useState<HTMLDivElement | null>(null);
  const { styles, attributes } = usePopper(
    cellRef.current,
    copiedPopperElement,
    {
      placement: "bottom-start",
      modifiers: [
        {
          name: "offset",
          options: { offset: [densityValues.paddingX - 4, 4] },
        },
      ],
    },
  );

  // Controls the editor popper -- the popper that shows the ObjectEditor for the cell
  const [editorPopper, setEditorPopper] = useState<HTMLDivElement | null>(null);
  const { styles: editorStyles, attributes: editorAttrs } = usePopper(
    cellRef.current,
    editorPopper,
    {
      placement: "bottom-start",
      modifiers: [
        {
          name: "offset",
          options: { offset: [0, -densityValues.height] },
        },
      ],
    },
  );
  const closeEditor = () => {
    setShowEditor(false);
    cellButtonRef.current?.focus();
    setPastedValue(undefined);
  };

  // When you click away from the cell, close the editor if it is open
  useClickAway({ current: editorPopper }, closeEditor);

  return (
    <>
      {/* TODO: Can we get rid of this wrapping div? */}
      <div
        ref={(r) => {
          if (cellRef.current !== r) {
            cellRef.current = r;
          }
          hotkeyRefs(r);
        }}
        className="relative flex h-full w-full items-center hover:bg-background-tertiary/75"
        style={{ width }}
      >
        {/* We do not use Button here because it's expensive and this table needs to be fast */}
        {/* eslint-disable-next-line react/forbid-elements */}
        <button
          data-testid="cell-editor-button"
          ref={cellButtonRef}
          className={classNames(
            // Show a border on the right side while animating to prevent the background highlight
            // from overlapping other cells
            didValueJustChange && "animate-highlight border-r",
            "font-mono text-xs text-content-primary",
            "w-full h-full flex items-center focus:outline-none",
            "focus:ring-1 focus:ring-border-selected text-left",
            "peer",
            isContextMenuOpen && "ring-1 ring-border-selected",
            !isEditable && "cursor-default",
          )}
          style={{
            padding: `${densityValues.paddingY}px ${densityValues.paddingX}px`,
          }}
          role={isEditable ? "button" : undefined}
          type="button"
          tabIndex={0}
          onKeyDown={arrowKeyHandler(cellRef)}
          onDoubleClick={clickHandler(isEditable, cellRef, editValue)}
        >
          {idReferenceLink !== undefined && columnName !== "_id" && (
            <Link2Icon className="mr-2 flex-none text-content-secondary" />
          )}
          <DataCellValue
            {...{
              isDateField,
              inferIsDate,
              value,
              isHovered: isHoveringCell,
              isReference: idReferenceLink !== undefined,
              detailHeader: columnName,
              stringValue,
            }}
          />
          {!column.disableResizing && (
            <div
              {...column.getResizerProps()}
              className="absolute right-0 top-0 inline-block h-full"
              style={{
                // @ts-expect-error bad typing in react-table
                ...column.getResizerProps().style,
                width: densityValues.paddingX,
              }}
            />
          )}
        </button>
        {/* Opens context menu, only visible on hover */}
        <Button
          data-testid="cell-context-menu-button"
          size="xs"
          variant="neutral"
          onClick={() =>
            cellRef.current &&
            contextMenuCallback({
              x: cellRef.current.getBoundingClientRect().right,
              y: cellRef.current.getBoundingClientRect().top,
            })
          }
          className={classNames(
            "absolute z-20 shadow-sm",
            isHoveringCell ? "block" : "hidden",
            "group peer-focus:block peer-focus:[focused]",
            "animate-none",
          )}
          style={{
            right:
              (cellRef.current?.clientWidth || 0) < 64
                ? -16
                : densityValues.paddingX - 4,
          }}
          icon={isHoveringCell && <DotsVerticalIcon />}
        >
          {!isHoveringCell && (
            <KeyboardShortcut
              value={["CtrlOrCmd", "Return"]}
              className="text-xs text-content-secondary"
            />
          )}
        </Button>
      </div>
      {/* Show a side panel to view the value of the current cell */}
      {showDetail && (
        <DataDetail
          value={value}
          header={
            <div className="flex items-center gap-1" data-testid="cell-detail">
              Viewing
              <span className="mr-2 font-mono">{columnName}</span>
              <span className="rounded border p-1 font-mono text-xs">
                Document: {rowId}
              </span>
            </div>
          }
          onClose={() => setShowDetail(false)}
        />
      )}
      {/* Show a side panel to view the entire document */}
      {showDocumentDetail && (
        <DataDetail
          value={document}
          header={
            <div
              className="flex items-center gap-1"
              data-testid="cell-detail-document"
            >
              Viewing
              <span className="rounded border p-1 font-mono text-xs">
                Document: {rowId}
              </span>
            </div>
          }
          onClose={() => setShowDocumentDetail(false)}
        />
      )}
      {/* Show the value editor popper right on top of the cell */}
      {showEditor && (
        <Portal>
          {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions */}
          <div
            ref={setEditorPopper}
            style={{
              ...editorStyles.popper,
              width,
            }}
            className="z-50 ml-[-1px] min-w-[24rem] animate-fadeInFromLoading"
            data-testid="cell-editor-popper"
            tabIndex={-1}
            onBlur={(e) => {
              if (e.relatedTarget === null) {
                closeEditor();
              }
            }}
            // for safari
            onKeyDown={async (e) => {
              if (e.key === "Escape") {
                closeEditor();
              }
            }}
            {...editorAttrs.popper}
          >
            <CellEditor
              validator={validator}
              shouldSurfaceValidatorErrors={shouldSurfaceValidatorErrors}
              allowTopLevelUndefined={allowTopLevelUndefined}
              onStopEditing={closeEditor}
              defaultValue={pastedValue}
              value={value}
              onSave={async (v) => {
                v !== undefined &&
                  (await patchDocument(tableName, rowId, columnName, v));
              }}
            />
          </div>
        </Portal>
      )}
      {/* Show confirmation dialog in production */}
      {showEnableProdEditsModal && (
        <ProductionEditsConfirmationDialog
          onClose={() => {
            setShowEnableProdEditsModal(false);
          }}
          onConfirm={async () => {
            onAuthorizeEdits && onAuthorizeEdits();
            setShowEnableProdEditsModal(false);
            setShowEditor(true);
          }}
        />
      )}
      {/* Show the popper when a value is copied */}
      <Transition
        show={didJustCopy !== null}
        as={Fragment}
        enter="transition-opacity ease-in-out duration-200"
        enterFrom="opacity-0"
        enterTo="opacity-100"
        leave="transition-opacity ease-in-out duration-200"
        leaveFrom="opacity-100"
        leaveTo="opacity-0"
      >
        <div
          ref={setCopiedPopperElement}
          style={styles.popper}
          className="z-50 flex items-center gap-1 rounded border bg-background-tertiary p-1 text-xs"
          data-testid="copied-popper"
          {...attributes.popper}
        >
          <CheckCircledIcon />
          Copied{" "}
          {didJustCopy && (
            <code>{didJustCopy === "value" ? columnName : "document"}</code>
          )}
        </div>
      </Transition>
    </>
  );
}

const clickHandler =
  (
    isEditable: boolean,
    cellRef: React.MutableRefObject<HTMLDivElement | null>,
    editValue: () => void,
  ) =>
  () => {
    if (isEditable) {
      editValue();
      return;
    }

    const selection = window.getSelection();
    selection?.selectAllChildren(cellRef.current!);
  };
