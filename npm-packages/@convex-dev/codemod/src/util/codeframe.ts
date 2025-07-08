import { codeFrameColumns } from "@babel/code-frame";
import { Node } from "ts-morph";

export function generateCodeframe(node: Node, message: string) {
  const sourceFile = node.getSourceFile();

  const nodeStart = sourceFile.getLineAndColumnAtPos(node.getStart());
  const nodeEnd = sourceFile.getLineAndColumnAtPos(node.getEnd());
  return codeFrameColumns(
    sourceFile.getText(),
    {
      start: {
        line: nodeStart.line,
        column: nodeStart.column,
      },
      end: {
        line: nodeEnd.line,
        column: nodeEnd.column,
      },
    },
    { highlightCode: true, message },
  );
}
