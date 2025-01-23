import { EyeNoneIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

export function SourceMissingPanel() {
  return (
    <div
      className={classNames(
        "flex h-full w-full flex-col items-center justify-center gap-5 min-w-[12rem]",
      )}
    >
      <EyeNoneIcon className={classNames("w-7 h-7")} />
      <p>We're unable to display your source code.</p>
    </div>
  );
}
