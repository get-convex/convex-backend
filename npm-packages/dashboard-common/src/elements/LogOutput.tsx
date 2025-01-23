import classNames from "classnames";
import { UdfLogOutput as Output } from "../lib/useLogs";
import { LogLevel } from "./LogLevel";

export function LogOutput({
  output,
  wrap,
}: {
  output: Output;
  wrap?: boolean;
}) {
  return (
    <div
      className={classNames(
        "text-xs overflow-y-auto text-content-secondary",
        wrap ? "whitespace-pre-wrap break-all" : "truncate",
      )}
    >
      {output.messages &&
        `${messagesToString(output)}${output.isTruncated ? " (truncated due to length)" : ""}`}
    </div>
  );
}

// Old version of LogOutput that's still used in some places.
// TODO: Replace with LogOutput.
export function LogLinesOutput({ output }: { output: Output[] }) {
  return output.length > 0 ? (
    <div className="flex flex-col divide-y border-b text-xs">
      {output.map((out, idx) => (
        <div className="flex items-start gap-2 p-2" key={idx}>
          <div className="flex">
            {out.level && (
              <span className="ml-auto rounded p-1">
                <LogLevel level={out.level} />
              </span>
            )}
          </div>
          <div className="max-w-5xl whitespace-pre-wrap p-1 text-content-primary">
            {`${messagesToString(out)}${out.isTruncated ? " (truncated due to length)" : ""}`}
          </div>
        </div>
      ))}
    </div>
  ) : null;
}

function messagesToString(output: Output): string {
  return output.messages
    .map((message) => {
      let newMessage: string = message;
      if (
        !output.isUnstructured &&
        message.startsWith("'") &&
        message.endsWith("'")
      ) {
        newMessage = slashUnescape(message);
      }
      return newMessage;
    })
    .join(" ");
}

const slashReplacements: Record<string, string> = {
  "\\\\": "\\",
  "\\n": "\n",
};

function slashUnescape(contents: string) {
  return contents.replace(/\\(\\|n)/g, (replace) => slashReplacements[replace]);
}
