import Editor, {
  BeforeMount,
  DiffEditorProps,
  EditorProps,
} from "@monaco-editor/react";
import { ValidatorJSON, Value } from "convex/values";
import { useTheme } from "next-themes";
import isEqual from "lodash/isEqual";
import React, { useCallback, useEffect, useRef, useState } from "react";

import isArray from "lodash/isArray";
import isPlainObject from "lodash/isPlainObject";
import { UNDEFINED_PLACEHOLDER } from "system-udfs/convex/_system/frontend/patchDocumentsFields";
import { captureMessage } from "@sentry/nextjs";
import { useMount } from "react-use";
import { useRouter } from "next/router";
import { stringifyValue } from "../../lib/stringifyValue";
import { cn } from "../../lib/cn";
import {
  ConvexSchemaValidationError,
  ConvexValidationError,
  LiteralNode,
} from "./ast/types";
import { walkAst, WalkAstOptions } from "./ast/walkAst";
import { registerIdCommands, useIdDecorations } from "./useIdDecorations";

const editorLineHeight = 18;

export type ObjectEditorProps = {
  defaultValue?: Value;
  onChange(v?: Value): void;
  onChangeInnerText?(v: string): void;
  onError(errors: string[]): void;
  path: string;
  className?: string;
  // Classes to apply to the Monaco editor.
  editorClassname?: string;
  // Classes to apply if the ObjectEditor has more than one line of code.
  multilineClasses?: string;
  fullHeight?: boolean;
  autoFocus?: boolean;
  saveAction?(): void;
  // If true, the save action will be triggered by pressing Enter in addition to Ctrl/Cmd+Enter.
  enterSaves?: boolean;
  // If enabled, the find (Cmd/Ctrl+F) action will be disabled.
  disableFind?: boolean;
  padding?: boolean;
  showLineNumbers?: boolean;
  disableFolding?: boolean;
  // If true, calls to onError will include errors produced by the validated.
  // In either case, the editor will still show the errors.
  shouldSurfaceValidatorErrors?: boolean;
  // Whether to show the full table name in the decoration for ID references.
  showTableNames?: boolean;
} & WalkAstOptions;

// Special case -- empty documents should be formatted to include space to entry a new field right away.
const emptyObject = "{\n\n}";

export function ObjectEditor(props: ObjectEditorProps) {
  const {
    className,
    editorClassname,
    multilineClasses,
    defaultValue,
    onChange,
    onChangeInnerText,
    onError,
    path,
    fullHeight = false,
    autoFocus = false,
    saveAction,
    disableFind = false,
    enterSaves = false,
    padding = true,
    showLineNumbers = false,
    disableFolding = false,
    validator,
    mode,
    shouldSurfaceValidatorErrors = false,
    showTableNames = false,
  } = props;

  const router = useRouter();

  const indentTopLevel = mode === "addDocuments" || mode === "editDocument";
  const [monaco, setMonaco] = useState<Parameters<BeforeMount>[0]>();

  const getDocumentRefs = useIdDecorations(monaco, path, showTableNames);

  // Initialize all markers on mount.
  useMount(() => {
    handleCodeChange(
      defaultValueString,
      mode,
      validator,
      "allowTopLevelUndefined" in props
        ? // eslint-disable-next-line react/destructuring-assignment
          !!props.allowTopLevelUndefined
        : false,
      shouldSurfaceValidatorErrors,
      handleError,
      onChange,
      getDocumentRefs,
    );
  });

  const saveActionRef = useRef(saveAction);
  useEffect(() => {
    saveActionRef.current = saveAction;
  }, [saveAction]);

  const handleError = useCallback(
    (errors: ConvexValidationError[]) => {
      const validationErrors = errors
        .filter(
          (e: any) =>
            shouldSurfaceValidatorErrors ||
            !(e instanceof ConvexSchemaValidationError),
        )
        .map((e: any) => e.message);
      onError?.(validationErrors);

      if (monaco) {
        setErrorMarkers(monaco, errors, path, shouldSurfaceValidatorErrors);
      }

      return validationErrors.length > 0;
    },
    [monaco, onError, path, shouldSurfaceValidatorErrors],
  );

  // Use state here so we don't recalculate the default value.
  const [defaultValueString] = useState(() => {
    if (defaultValue === undefined) {
      return "";
    }
    if (isEqual(defaultValue, {})) {
      return emptyObject;
    }
    if (isEqual(defaultValue, [{}])) {
      return `[${emptyObject}]`;
    }
    return stringifyValue(defaultValue, true, indentTopLevel);
  });

  const numLinesFromCode = (code: string) => code.split("\n").length + 1;

  const [numLines, setNumLines] = useState(
    numLinesFromCode(defaultValueString),
  );
  const editorHeight =
    numLines <= 2 ? 18 * 2 : Math.min(numLines, 15) * editorLineHeight;

  const handleChange = useCallback(
    (code?: string) => {
      // Hook to inform the parent component of the inner text of the editor.
      onChangeInnerText && onChangeInnerText(code ?? "");

      setNumLines(code ? numLinesFromCode(code) : 1);
      handleCodeChange(
        code,
        mode,
        validator,
        "allowTopLevelUndefined" in props
          ? // eslint-disable-next-line react/destructuring-assignment
            !!props.allowTopLevelUndefined
          : false,
        shouldSurfaceValidatorErrors,
        handleError,
        onChange,
        getDocumentRefs,
      );
    },
    [
      onChangeInnerText,
      mode,
      validator,
      props,
      shouldSurfaceValidatorErrors,
      handleError,
      onChange,
      getDocumentRefs,
    ],
  );

  const { resolvedTheme: currentTheme } = useTheme();
  const prefersDark = currentTheme === "dark";
  return (
    <div
      data-testid="objectEditor"
      className={cn(
        // Setting a min-h makes sure the editor is able to properly resize when the
        // parent is resized.
        "border rounded min-h-4 w-full h-full",
        className,
        numLines > 2 && multilineClasses,
      )}
      style={{
        minHeight: 16,
        maxHeight: "100%",
        height: fullHeight ? "100%" : editorHeight,
      }}
      onScroll={(e) => e.stopPropagation()}
    >
      <Editor
        height="100%"
        className={cn(padding && "pt-2", editorClassname)}
        defaultLanguage="javascript"
        defaultValue={defaultValueString}
        options={{
          ...editorOptions,
          ...(showLineNumbers
            ? {
                lineNumbers: "on",
                lineNumbersMinChars: 5,
                lineDecorationsWidth: 10,
              }
            : {}),
          folding: !disableFolding,
          theme: prefersDark ? "vs-dark" : "vs",
        }}
        // Never make the path look like a URI scheme.
        path={path.replace(":", "_")}
        onChange={handleChange}
        beforeMount={(m) => {
          m.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
            // Disable all syntax validation and use the results from AST parsing instead.
            noSemanticValidation: true,
            noSyntaxValidation: true,
            noSuggestionDiagnostics: true,
            // The "unused block" diagnostic code.
            diagnosticCodesToIgnore: [7028],
          });
          setMonaco(m);
        }}
        onMount={(editor, m) => {
          registerIdCommands(m, router);

          if (disableFind) {
            editor.addAction({
              id: "find",
              label: "find",
              keybindings: [m.KeyMod.CtrlCmd | m.KeyCode.KeyF],
              run: () => {},
            });
          }

          if (saveAction) {
            const keybindings = [m.KeyMod.CtrlCmd | m.KeyCode.Enter];
            if (enterSaves) {
              keybindings.push(m.KeyCode.Enter);
            }
            editor.addAction({
              id: "saveAction",
              label: "Save value",
              keybindings,
              run() {
                saveActionRef.current?.();
              },
            });
          }

          if (!autoFocus) {
            return;
          }
          editor.focus();

          const code = editor.getValue();
          if (!code) {
            return;
          }
          const codeLines = code.trimEnd().split("\n");
          let lastLine = codeLines.pop();

          let isMultiLineObject = false;
          if (lastLine === "}") {
            lastLine = codeLines.pop();
            isMultiLineObject = true;
          }
          if (lastLine === undefined) {
            return;
          }

          // Pick the location to place the cursor based on the type of the value.
          const column =
            isPlainObject(defaultValue) ||
            isArray(defaultValue) ||
            (typeof defaultValue === "string" && !isMultiLineObject)
              ? // Arrays and objects have end braces and strings have end quotes, so we want to place the cursor before those.
                lastLine.length
              : // All other types are not wrapped with anything, so we want to place the cursor after the value.
                lastLine.length + 1;
          editor.setPosition({
            // Objects rendered on multiple lines should have the last line be set to the line before the closing brace.
            lineNumber: code.split("\n").length - (isMultiLineObject ? 1 : 0),
            column,
          });
        }}
        loading={null}
      />
    </div>
  );
}

function setErrorMarkers(
  monaco: Parameters<BeforeMount>[0],
  errors: ConvexValidationError[],
  path: string,
  shouldSurfaceValidatorErrors: boolean,
) {
  const markers = [
    ...errors
      .filter((e) => e instanceof ConvexValidationError && e.loc !== undefined)
      .map((e) => ({
        message: e.message,
        severity:
          !shouldSurfaceValidatorErrors &&
          e instanceof ConvexSchemaValidationError
            ? monaco.MarkerSeverity.Warning
            : monaco.MarkerSeverity.Error,

        startLineNumber: e.loc?.start.line ?? 0,
        // Looks like Monaco counts from 1 and Acorn counts from 0
        startColumn: (e.loc?.start.column ?? 0) + 1,
        endLineNumber: e.loc?.end.line ?? 0,
        // Looks like Monaco counts from 1 and Acorn counts from 0
        endColumn: (e.loc?.end.column ?? 0) + 1,
        ...e.markerData,
      })),
  ];
  const model = monaco.editor
    .getModels()
    ?.find((m) => path.replace(":", "_") === m.uri.path.slice(1));
  if (!model) {
    captureMessage(`Model not found in Monaco editor for path: ${path}`);
    return;
  }
  monaco.editor.setModelMarkers(model, "", markers);
}

function handleCodeChange(
  code: string | undefined,
  mode: "editField" | "addDocuments" | "editDocument" | "patchDocuments",
  validator: ValidatorJSON | undefined,
  allowTopLevelUndefined: boolean,
  shouldSurfaceValidatorErrors: boolean,
  handleError: (errors: ConvexValidationError[]) => boolean,
  onChange: (result: Value) => void,
  getDocumentRefs: (ids: LiteralNode[]) => void,
) {
  try {
    const {
      value: result,
      errors: astErrors,
      ids: newIds,
    } = processCode(
      code,
      mode,
      validator,
      allowTopLevelUndefined,
      shouldSurfaceValidatorErrors,
    );
    const hasSurfacedErrors = handleError(astErrors);
    !hasSurfacedErrors && onChange(result);
    getDocumentRefs(newIds);
  } catch (e: any) {
    if (e instanceof SyntaxError) {
      const result = e.message.match(/(.*) \(([0-9]+):([0-9]+)\)/);
      if (result) {
        const message = result[1];
        const line = parseInt(result[2]);
        const column = parseInt(result[3]) + 1;
        const position = { line, column, offset: 0 };
        handleError([
          new ConvexValidationError(message, {
            start: position,
            end: position,
          }),
        ]);
        return;
      }
    }
    throw e;
  }
}

function processCode(
  code: string | undefined,
  mode: "editField" | "addDocuments" | "editDocument" | "patchDocuments",
  validator: ValidatorJSON | undefined,
  allowTopLevelUndefined: boolean,
  shouldSurfaceValidatorErrors: boolean,
) {
  return code
    ? walkAst(
        code,
        mode === "editField"
          ? {
              validator,
              mode,
              allowTopLevelUndefined,
            }
          : {
              validator,
              mode,
            },
      )
    : {
        value: UNDEFINED_PLACEHOLDER,
        errors:
          // Edge case: Here, we have to handle the case where the top level is not a document, like in CellEditor.
          // In this case, we do not want to allow the user to enter undefined if their field is not optional.
          mode === "editField" &&
          !allowTopLevelUndefined &&
          validator &&
          shouldSurfaceValidatorErrors
            ? [
                new ConvexSchemaValidationError(
                  "RequiredPropertyMissing",
                  validator,
                  undefined,
                ),
              ]
            : [],
        ids: [],
      };
}

export const editorOptions: EditorProps["options"] &
  DiffEditorProps["options"] = {
  tabFocusMode: false,
  automaticLayout: true,
  minimap: { enabled: false },
  overviewRulerBorder: false,
  scrollBeyondLastLine: false,
  find: {
    addExtraSpaceOnTop: false,
    autoFindInSelection: "never",
    seedSearchStringFromSelection: "never",
  },
  lineNumbers: "off",
  glyphMargin: false,
  lineDecorationsWidth: 0,
  lineNumbersMinChars: 0,
  scrollbar: {
    alwaysConsumeMouseWheel: false,
    horizontalScrollbarSize: 8,
    verticalScrollbarSize: 8,
    useShadows: false,
    vertical: "visible",
  },
  suggest: { preview: false },
  hideCursorInOverviewRuler: true,
  quickSuggestions: false,
  parameterHints: { enabled: false },
  suggestOnTriggerCharacters: false,
  snippetSuggestions: "none",
  contextmenu: false,
  codeLens: false,
  disableLayerHinting: true,
  inlayHints: { enabled: "off" },
  inlineSuggest: { enabled: false },
  lightbulb: { enabled: false },
  hover: { above: false },
  guides: {
    bracketPairs: false,
    bracketPairsHorizontal: false,
    highlightActiveBracketPair: false,
    indentation: false,
    highlightActiveIndentation: false,
  },
  bracketPairColorization: { enabled: false },
  matchBrackets: "never",
  tabCompletion: "off",
  selectionHighlight: false,
  occurrencesHighlight: false,
  renderLineHighlight: "none",
};
