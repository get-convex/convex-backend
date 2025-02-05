import { CopyIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { useState } from "react";
import { copyTextToClipboard } from "@common/lib/utils";
import { Button } from "@common/elements/Button";

export function CopyTextButton({
  text,
  className,
}: {
  text: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  const copyText = () => {
    let canceled = false;
    void (async () => {
      if (text) {
        await copyTextToClipboard(text);
        if (canceled) return;
        setCopied(true);

        setTimeout(() => {
          if (canceled) return;
          setCopied(false);
        }, 1000);
      }
    })();
    return () => {
      canceled = true;
    };
  };

  return (
    <div className="group flex items-center gap-1">
      <Button
        variant="neutral"
        size="xs"
        className={classNames("text-xs transition-all", className)}
        onClick={copyText}
      >
        <span>{text}</span>
      </Button>
      {!copied ? (
        <Button variant="unstyled" onClick={copyText}>
          <CopyIcon className={classNames("hidden group-hover:block")} />
        </Button>
      ) : (
        <span className="text-xs">Copied!</span>
      )}
    </div>
  );
}
