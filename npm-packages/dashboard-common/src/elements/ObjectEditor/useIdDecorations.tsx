import { BeforeMount } from "@monaco-editor/react";
import { useQuery } from "convex/react";
import { useTheme } from "next-themes";
import { useState, useEffect } from "react";
import udfs from "udfs";
import { useRouter } from "next/router";
import { cn } from "lib/cn";
import { GenericDocument } from "convex/server";
import { SourceLocation } from "acorn";
import { captureMessage } from "@sentry/nextjs";
import type { editor } from "monaco-editor/esm/vs/editor/editor.api";
import { stringifyValue } from "dashboard-common";
import {
  copyTextToClipboard,
  documentHref,
  getReferencedTableName,
  toast,
} from "../../lib/utils";
import { useNents } from "../../lib/useNents";
import { LiteralNode } from "./ast/types";

export function useIdDecorations(
  monaco: Parameters<BeforeMount>[0] | undefined,
  path: string,
  showTableNames: boolean = true,
): (ids: LiteralNode[]) => void {
  const router = useRouter();
  const { resolvedTheme: currentTheme } = useTheme();
  const prefersDark = currentTheme === "dark";
  const [ids, setIds] = useState<LiteralNode[]>([]);

  const componentId = useNents().selectedNent?.id ?? null;
  const tableMapping = useQuery(udfs.getTableMapping.default, {
    componentId,
  });

  // TODO: Instead of getting the table name from the table mapping, we should infer it from a validator.
  const idsArg = ids
    .map((id) => ({
      id: id.value,
      tableName: getReferencedTableName(tableMapping, id.value),
    }))
    .filter(
      (id): id is { id: string; tableName: string } => id.tableName !== null,
    );

  // Dedupe ids found in the document.
  const uniqueIdsArg = Array.from(new Set(idsArg.map((id) => id.id))).map(
    (id) => idsArg.find((arg) => arg.id === id)!,
  );

  // Lookup all documents by their ids.
  const docs = useQuery(
    udfs.listById.default,
    uniqueIdsArg.length
      ? {
          componentId,
          ids: uniqueIdsArg,
        }
      : "skip",
  );

  useEffect(() => {
    if (!docs || !monaco) {
      return;
    }

    const model = monaco?.editor
      .getModels()
      ?.find((m) => path.replace(":", "_") === m.uri.path.slice(1));
    if (!model) {
      captureMessage(`Model not found in Monaco editor for path: ${path}`);
      return;
    }

    const documentRefs = ids.map((id) => ({
      doc: docs.find((doc) => doc && doc._id === id.value) ?? null,
      id: id.value as string,
      loc: id.loc as SourceLocation,
    }));

    let decorationIds: string[] | undefined;
    const doProcessDecorations = async () => {
      decorationIds = await processDecorations({
        documentRefs,
        tableMapping,
        prefersDark,
        componentId,
        showTableNames,
        monaco,
        model,
      });
    };
    void doProcessDecorations();

    // Remove existing decorations on cleanup
    return () => {
      if (decorationIds) {
        model.deltaDecorations(decorationIds, []);
      }
    };
  }, [
    componentId,
    docs,
    ids,
    monaco,
    path,
    prefersDark,
    router,
    showTableNames,
    tableMapping,
  ]);

  return setIds;
}

function hoverMessageForDoc(
  tableName: string,
  prefersDark: boolean,
  doc: GenericDocument,
  id: string,
  componentId: string | null,
  message: string,
) {
  return [
    {
      value: `Document in ${colorizeHelperText(tableName, prefersDark)}, created ${colorizeHelperText(new Date(doc._creationTime as number).toLocaleString(), prefersDark)}`,
      supportHtml: true,
      isTrusted: true,
    },
    {
      value: `${createMarkdownLink(
        "Open in new tab",
        GO_TO_DOCUMENT_COMMAND,
        "codicon-link-external",
        { id, tableName, componentId },
      )}
      &nbsp;&nbsp;&nbsp;&nbsp;
      ${createMarkdownLink(
        "Copy Document",
        COPY_DOCUMENT_COMMAND,
        "codicon-copy",
        {
          docString: stringifyValue(doc, true),
        },
      )}`,
      supportHtml: true,
      isTrusted: {
        enabledCommands: [GO_TO_DOCUMENT_COMMAND, COPY_DOCUMENT_COMMAND],
      },
    },
    {
      value: message,
      supportHtml: true,
      isTrusted: true,
    },
  ];
}

async function colorizeDoc(
  doc: GenericDocument,
  prefersDark: boolean,
  monaco: Parameters<BeforeMount>[0],
) {
  return `${await monaco.editor.colorize(
    stringifyValue(doc, true, true),
    "javascript",
    {},
  )}`.replace(
    // Replace class name with inline style
    /class="([^"]+)"/g,
    (match, className) =>
      `style="${styleForClass(prefersDark, className) || ""}"`,
  );
}

async function processDecorations({
  documentRefs,
  tableMapping,
  prefersDark,
  componentId,
  showTableNames,
  monaco,
  model,
}: {
  documentRefs: Array<{
    doc: GenericDocument | null;
    loc: SourceLocation;
    id: string;
  }>;
  tableMapping: Record<number, string>;
  prefersDark: boolean;
  componentId: string | null;
  showTableNames: boolean;
  monaco: Parameters<BeforeMount>[0];
  model: editor.ITextModel;
}) {
  const newDecorations = await Promise.all(
    documentRefs
      .filter(({ loc }) => loc !== undefined)
      .map(({ doc, loc, id }) =>
        provideDecoration({
          doc,
          loc,
          id,
          tableMapping,
          prefersDark,
          componentId,
          showTableNames,
          monaco,
        }),
      ),
  );
  return model.deltaDecorations([], newDecorations);
}

async function provideDecoration({
  doc,
  loc,
  id,
  tableMapping,
  prefersDark,
  componentId,
  showTableNames,
  monaco,
}: {
  doc: GenericDocument | null;
  loc: SourceLocation;
  id: string;
  tableMapping: Record<number, string>;
  prefersDark: boolean;
  componentId: string | null;
  showTableNames: boolean;
  monaco: Parameters<BeforeMount>[0];
}): Promise<editor.IModelDeltaDecoration> {
  const tableName = getReferencedTableName(tableMapping, id) as string;

  return {
    options: {
      hoverMessage: doc
        ? hoverMessageForDoc(
            tableName,
            prefersDark,
            doc,
            id,
            componentId,
            await colorizeDoc(doc, prefersDark, monaco),
          )
        : [
            {
              value: `Document not found. It may be deleted, from another component, or another Convex deployment.`,
            },
          ],
      afterContentClassName: cn(
        "ml-1 mr-1 hover-decoration",
        doc ? "codicon-link mtk23" : "codicon-warning mtk11",
      ),
      after: {
        content: showTableNames ? `Id<"${tableName}">` : "Id",
        inlineClassName: cn(doc ? "mtk23" : "mtk11", "mtki"),
        cursorStops: monaco.editor.InjectedTextCursorStops.None,
      },
    },

    range: new monaco.Range(
      loc.end.line,
      loc.end.column,
      loc.end.line,
      loc.end.column + 1,
    ),
  };
}

function createMarkdownLink(
  label: string,
  cmd: string,
  icon: string,
  args: object,
): string {
  const encodedArgs = encodeURIComponent(JSON.stringify(args));
  return `[${label} <span class="codicon ${icon}"></span>](command:${cmd}?${encodedArgs} "${label}")`;
}

function colorizeHelperText(text: string, prefersDark: boolean) {
  return `<code><span style="${styleForClass(prefersDark, "mtk23")}">${text}</span></code>`;
}

const GO_TO_DOCUMENT_COMMAND = "goToDocument";
const COPY_DOCUMENT_COMMAND = "copyDocument";

export function registerIdCommands(
  monaco: Parameters<BeforeMount>[0],
  router: ReturnType<typeof useRouter>,
) {
  monaco.editor.registerCommand(
    GO_TO_DOCUMENT_COMMAND,
    (
      accessor,
      args: {
        id: string;
        tableName: string;
        componentId: string | null;
      },
    ) => {
      const href = documentHref(router, args.tableName, args.id);
      const query = `${href.query.component ? `component=${href.query.component}&` : ""}table=${href.query.table}&filters=${href.query.filters}`;
      const newPage = `${window.location.pathname.split("/").slice(0, 5).join("/")}/data?${query}`;
      window.open(`${window.location.origin + newPage}`, "_blank");
    },
  );

  monaco.editor.registerCommand(
    COPY_DOCUMENT_COMMAND,
    (
      accessor,
      args: {
        docString: string;
      },
    ) => {
      void copyTextToClipboard(args.docString);
      toast("success", "Document copied to clipboard.");
    },
  );
}

// When text is colorized, monaco applies classnames to elements it will render. However, monaco also strips class names when rendering the text in hover,
// so we hardcode the exacty color style for the classes we'll be using here.
function styleForClass(darkMode: boolean, cls: string) {
  const color = classes[cls]?.[darkMode ? "dark" : "light"];
  return color ? `color:${color};` : "";
}

const classes: { [key: string]: { dark: string; light: string } } = {
  mtk1: { dark: "#d4d4d4", light: "#000000" },
  mtk2: { dark: "#1e1e1e", light: "#fffffe" },
  mtk3: { dark: "#cc6666", light: "#808080" },
  mtk4: { dark: "#9cdcfe", light: "#ff0000" },
  mtk5: { dark: "#ce9178", light: "#0451a5" },
  mtk6: { dark: "#b5cea8", light: "#0000ff" },
  mtk7: { dark: "#608b4e", light: "#098658" },
  mtk8: { dark: "#569cd6", light: "#008000" },
  mtk9: { dark: "#dcdcdc", light: "#dd0000" },
  mtk10: { dark: "#808080", light: "#383838" },
  mtk11: { dark: "#f44747", light: "#cd3131" },
  mtk12: { dark: "#c586c0", light: "#863b00" },
  mtk13: { dark: "#a79873", light: "#af00db" },
  mtk14: { dark: "#dd6a6f", light: "#800000" },
  mtk15: { dark: "#5bb498", light: "#e00000" },
  mtk16: { dark: "#909090", light: "#3030c0" },
  mtk17: { dark: "#778899", light: "#666666" },
  mtk18: { dark: "#ff00ff", light: "#778899" },
  mtk19: { dark: "#b46695", light: "#c700c7" },
  mtk20: { dark: "#ff0000", light: "#a31515" },
  mtk21: { dark: "#4f76ac", light: "#4f76ac" },
  mtk22: { dark: "#3dc9b0", light: "#008080" },
  mtk23: { dark: "#74b0df", light: "#001188" },
  mtk24: { dark: "#4864aa", light: "#4864aa" },
};
