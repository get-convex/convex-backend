import classNames from "classnames";
import { CopyButton } from "@common/elements/CopyButton";

// Pass a list of words/tokens to prevent line-wrapping in the middle
// of them, like in `--foo=bar`.
export function CodeLine({
  className,
  code,
}: {
  className?: string;
  code: string;
}) {
  return (
    <div className="relative flex flex-row items-stretch">
      <code
        className={classNames(
          "static bg-background-secondary border rounded-sm p-5 pr-32 flex flex-row w-full",
          className,
        )}
      >
        <span className="mr-2 text-content-secondary select-none">$ </span>
        <div
          className="text-content-primary"
          style={{
            lineHeight: 1.5,
          }}
        >
          {code}
        </div>
      </code>
      <div className="absolute top-0 right-0 h-10">
        <CopyButton text={code} />
      </div>
    </div>
  );
}
