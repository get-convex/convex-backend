import classNames from "classnames";
import { CopyButton } from "dashboard-common";

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
          "static bg-background-secondary border rounded p-5 pr-32 flex flex-row w-full",
          className,
        )}
      >
        <span className="mr-2 select-none text-content-secondary">$ </span>
        <div
          className="text-content-primary"
          style={{
            lineHeight: 1.5,
          }}
        >
          {code}
        </div>
      </code>
      <div className="absolute right-0 top-0 h-10">
        <CopyButton text={code} />
      </div>
    </div>
  );
}
