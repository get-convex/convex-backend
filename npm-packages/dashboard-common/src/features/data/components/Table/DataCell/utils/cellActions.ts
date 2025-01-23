import React, { MutableRefObject, useCallback, useState } from "react";
import { useDebounce } from "react-use";
import { useHotkeys } from "react-hotkeys-hook";
import { RefType } from "react-hotkeys-hook/dist/types";
import { UrlObject } from "url";

import { GenericDocument } from "convex/server";
import { Value } from "convex/values";

import {
  stringifyValue,
  copyTextToClipboard,
  useNents,
} from "dashboard-common";

import { Target } from "../../../ContextMenu";
import { useContextMenuTrigger } from "../../../../lib/useContextMenuTrigger";

import { useIdReferenceLink } from "./useIdReferenceLink";

// Handles most of the logic for interacting with a cell in the table.
// This includes opening the context menu, copying values, editing values, etc.
export function useCellActions({
  cellRef,
  onOpenContextMenu,
  onCloseContextMenu,
  columnName,
  rowId,
  value,
  document,
  areEditsAuthorized,
  canManageTable,
  editDocument,
  onAuthorizeEdits,
  setPastedValue,
  setShowEditor,
  setShowEnableProdEditsModal,
  setShowDetail,
  setShowDocumentDetail,
}: {
  cellRef: MutableRefObject<HTMLElement | null>;
  onOpenContextMenu: OpenContextMenu;
  onCloseContextMenu: () => void;
  columnName: string;
  rowId: string | null;
  value: Value;
  document: GenericDocument;
  areEditsAuthorized: boolean;
  editDocument: () => void;
  onAuthorizeEdits?: () => void;
  canManageTable: boolean;
  setPastedValue: React.Dispatch<React.SetStateAction<Value | undefined>>;
  setShowEditor: React.Dispatch<React.SetStateAction<boolean>>;
  setShowEnableProdEditsModal: React.Dispatch<React.SetStateAction<boolean>>;
  setShowDetail: React.Dispatch<React.SetStateAction<boolean>>;
  setShowDocumentDetail: React.Dispatch<React.SetStateAction<boolean>>;
}) {
  const { selectedNent } = useNents();
  const isInUnmountedComponent = !!(
    selectedNent && selectedNent.state !== "active"
  );

  const disableEdit =
    columnName.startsWith("_") || !canManageTable || isInUnmountedComponent;

  const idReferenceLink = useIdReferenceLink(value, columnName);
  const [didJustCopy, setDidJustCopy] = useState<"document" | "value" | null>(
    null,
  );
  useDebounce(() => setDidJustCopy(null), 1000, [didJustCopy]);

  const copyValue = useCallback(async () => {
    await copyTextToClipboard(
      typeof value === "string" ? value : stringifyValue(value, true),
    );
    setDidJustCopy("value");
  }, [value]);

  const copyDocument = useCallback(async () => {
    await copyTextToClipboard(stringifyValue(document, true));
    setDidJustCopy("document");
  }, [document]);

  const editValue = useCallback(
    (v?: Value) => {
      if (disableEdit) {
        return;
      }
      if (areEditsAuthorized) {
        v !== undefined && setPastedValue(v);
        setShowEditor(true);
      } else if (onAuthorizeEdits) {
        setShowEnableProdEditsModal(true);
      }
    },
    [
      areEditsAuthorized,
      disableEdit,
      onAuthorizeEdits,
      setPastedValue,
      setShowEditor,
      setShowEnableProdEditsModal,
    ],
  );

  const goToDoc = useCallback(() => {
    if (idReferenceLink) {
      const currentUrl = new URL(window.location.href);
      const linkUrl = new URL(idReferenceLink.pathname!, currentUrl.origin);
      linkUrl.search = new URLSearchParams(
        idReferenceLink.query as Record<string, string>,
      ).toString();
      window.open(linkUrl.toString(), "_blank");
    }
  }, [idReferenceLink]);

  const viewValue = useCallback(() => {
    if (!idReferenceLink) {
      setShowDetail(true);
    }
  }, [idReferenceLink, setShowDetail]);

  const viewDocument = useCallback(() => {
    setShowDocumentDetail(true);
  }, [setShowDocumentDetail]);

  const contextMenuCallback = useCallback(
    (position: Target) =>
      onOpenContextMenu(position, rowId, {
        column: columnName,
        value,
        callbacks: {
          view: viewValue,
          viewDoc: viewDocument,
          copy: copyValue,
          copyDoc: copyDocument,
          edit: editValue,
          // Assume invariant:
          // Edit document should have all the necessary permissions checks
          // since it is passed to DataCell from the Table component.
          editDoc: editDocument,
          goToRef: goToDoc,
          docRefLink: idReferenceLink,
        },
      }),
    [
      onOpenContextMenu,
      rowId,
      columnName,
      value,
      idReferenceLink,
      viewValue,
      viewDocument,
      copyDocument,
      goToDoc,
      copyValue,
      editValue,
      editDocument,
    ],
  );

  useContextMenuTrigger(cellRef, contextMenuCallback, onCloseContextMenu);

  return {
    didJustCopy,
    idReferenceLink,
    copyValue,
    copyDocument,
    editValue,
    goToDoc,
    viewValue,
    viewDocument,
    contextMenuCallback,
  };
}

export type ActionHotkeysProps = {
  copyCb: () => void;
  copyDocCb: () => void;
  viewCb: () => void;
  viewDocCb: () => void;
  editCb: () => void;
  editDocCb: () => void;
  goToDocCb: () => void;
  openContextMenu?: () => void;
};

export function useActionHotkeys({
  copyCb,
  copyDocCb,
  viewCb,
  viewDocCb,
  editCb,
  editDocCb,
  goToDocCb,
  openContextMenu,
}: ActionHotkeysProps) {
  const refs = [
    useHotkeys<HTMLDivElement>(["meta+c", "ctrl+c"], wrapper(copyCb)),
    useHotkeys<HTMLDivElement>(
      ["meta+shift+c", "ctrl+shift+c"],
      wrapper(copyDocCb),
    ),
    useHotkeys<HTMLDivElement>(["space"], wrapper(viewCb)),
    useHotkeys<HTMLDivElement>(["shift+space"], wrapper(viewDocCb)),
    useHotkeys<HTMLDivElement>(["return"], wrapper(editCb)),
    useHotkeys<HTMLDivElement>(["shift+return"], wrapper(editDocCb)),
    useHotkeys<HTMLDivElement>(["meta+g", "ctrl+g"], wrapper(goToDocCb)),
    useHotkeys<HTMLDivElement>(
      ["meta+return", "ctrl+return"],
      openContextMenu || (() => {}),
    ),
  ];

  return mergeHotkeyRefs(...refs);
}

const mergeHotkeyRefs =
  (...refs: MutableRefObject<RefType<HTMLDivElement>>[]) =>
  (node: RefType<HTMLDivElement>) => {
    for (const ref of refs) {
      ref.current = node;
    }
  };

const wrapper = (cb: () => void) => (e: KeyboardEvent) => {
  e.preventDefault();
  cb();
};

export type SelectedCell = {
  column: string;
  value: Value | undefined;
  callbacks?: {
    copy: () => void;
    copyDoc: () => void;
    goToRef: () => void;
    edit: () => void;
    editDoc: () => void;
    view: () => void;
    viewDoc: () => void;
    docRefLink: UrlObject | undefined;
  };
} | null;

export type TableContextMenuState = {
  target: Target;
  selectedCell:
    | (SelectedCell & {
        rowId: string | null;
      })
    | null;
};

export type OpenContextMenu = (
  position: { x: number; y: number },
  rowId: string | null,
  cell: SelectedCell,
) => void;
