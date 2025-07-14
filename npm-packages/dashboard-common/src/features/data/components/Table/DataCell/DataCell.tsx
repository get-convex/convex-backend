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
import { useTableDensity } from "@common/features/data/lib/useTableDensity";

import { ProductionEditsConfirmationDialog } from "@common/elements/ProductionEditsConfirmationDialog";

import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { DataDetail } from "@common/features/data/components/Table/DataCell/DataDetail";
import { CellEditor } from "@common/features/data/components/Table/DataCell/CellEditor";
import { DataCellValue } from "@common/features/data/components/Table/DataCell/DataCellValue";

import type { usePatchDocumentField } from "@common/features/data/components/Table/utils/usePatchDocumentField";
import { arrowKeyHandler } from "@common/features/data/components/Table/utils/arrowKeyHandler";
import {
  OpenContextMenu,
  useActionHotkeys,
  useCellActions,
} from "@common/features/data/components/Table/DataCell/utils/cellActions";
import { usePasteListener } from "@common/features/data/components/Table/DataCell/utils/usePasteListener";
import { useTrackCellChanges } from "@common/features/data/components/Table/DataCell/utils/useTrackCellChanges";
import { useValidator } from "@common/features/data/components/Table/DataCell/utils/useValidator";
import { SchemaJson } from "@common/lib/format";
import { stringifyValue } from "@common/lib/stringifyValue";
import { buttonClasses } from "@ui/Button";
import { Loading } from "@ui/Loading";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { useNents } from "@common/lib/useNents";
import { getReferencedTableName } from "@common/lib/utils";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { Tooltip } from "@ui/Tooltip";
import { cn } from "@ui/cn";

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
  const [isFocused, setIsFocused] = useState(false);
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
            "w-full h-full flex items-center focus:outline-hidden",
            "focus:ring-1 focus:ring-border-selected text-left",
            isContextMenuOpen && "ring-1 ring-border-selected",
            !isEditable && "cursor-default",
          )}
          style={{
            padding: `${densityValues.paddingY}px ${densityValues.paddingX}px`,
          }}
          role={isEditable ? "button" : undefined}
          type="button"
          tabIndex={0}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          onKeyDown={arrowKeyHandler(cellRef)}
          onDoubleClick={clickHandler(isEditable, cellRef, editValue)}
        >
          {idReferenceLink !== undefined && columnName !== "_id" && (
            <Tooltip
              tip={<DocumentPreview id={value} />}
              contentClassName="bg-background-secondary animate-fadeInFromLoading"
              maxWidthClassName="max-w-[22rem]"
              delayDuration={250}
              wrapsButton
            >
              <Link2Icon className="mr-2 flex-none text-content-secondary" />
            </Tooltip>
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
              className="absolute top-0 right-0 inline-block h-full"
              style={{
                // @ts-expect-error bad typing in react-table
                ...column.getResizerProps().style,
                width: densityValues.paddingX,
              }}
            />
          )}
        </button>
        {(isHoveringCell || isFocused) && (
          // eslint-disable-next-line react/forbid-elements
          <button
            data-testid="cell-context-menu-button"
            type="button"
            onClick={() =>
              cellRef.current &&
              contextMenuCallback({
                x: cellRef.current.getBoundingClientRect().right,
                y: cellRef.current.getBoundingClientRect().top,
              })
            }
            className={cn(
              buttonClasses({ size: "xs", variant: "neutral" }),
              "absolute z-20 shadow-xs hover:bg-background-tertiary",
              isFocused && "focused",
              "animate-none",
            )}
            style={{
              right:
                (cellRef.current?.clientWidth || 0) < 64
                  ? -16
                  : densityValues.paddingX - 4,
            }}
          >
            {isHoveringCell && <DotsVerticalIcon />}
            {!isHoveringCell && (
              <KeyboardShortcut
                value={["CtrlOrCmd", "Return"]}
                className="text-xs text-content-secondary"
              />
            )}
          </button>
        )}
      </div>
      {/* Show a side panel to view the value of the current cell */}
      {showDetail && (
        <DataDetail
          value={value}
          header={
            <div className="flex items-center gap-1" data-testid="cell-detail">
              Viewing
              <span className="mr-2 font-mono">{columnName}</span>
              <span className="rounded-sm border p-1 font-mono text-xs">
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
              <span className="rounded-sm border p-1 font-mono text-xs">
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
          className="z-50 flex items-center gap-1 rounded-sm border bg-background-tertiary p-1 text-xs"
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

function DocumentPreview({ id }: { id: string | Value }) {
  // Safely convert id to string if it's not already
  const stringId = typeof id === "string" ? id : String(id);

  const componentId = useNents().selectedNent?.id ?? null;
  const tableMapping = useQuery(udfs.getTableMapping.default, {
    componentId,
  });
  const tableName = getReferencedTableName(tableMapping, stringId);

  const docs = useQuery(udfs.listById.default, {
    componentId,
    ids: [{ id: stringId, tableName: tableName ?? "" }],
  });

  if (!docs) {
    return <Loading className="h-8 w-80" />;
  }

  if (!docs?.[0]) {
    return <div>Document not found.</div>;
  }

  return (
    <div className="w-80">
      <ReadonlyCode
        disableLineNumbers
        code={stringifyValue(docs[0] ?? null, true, true)}
        path={`documentPreview-${stringId}`}
        height={{ type: "content", maxHeightRem: 20 }}
      />
    </div>
  );
}
