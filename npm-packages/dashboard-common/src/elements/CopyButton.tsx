import React, { useState } from "react";
import { CopyIcon } from "@radix-ui/react-icons";
import { copyTextToClipboard } from "lib/utils";
import { Button } from "elements/Button";

export function CopyButton({
  text,
  className,
  inline = false,
  tip,
  tipSide,
  disabled,
}: {
  text: string;
  className?: string;
  inline?: boolean;
  tip?: string;
  tipSide?: "top" | "bottom" | "left" | "right";
  disabled?: boolean;
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
        }, 2000);
      }
    })();
    return () => {
      canceled = true;
    };
  };

  return (
    <Button
      onClick={copyText}
      size="xs"
      icon={<CopyIcon />}
      variant="neutral"
      inline={inline}
      className={className}
      tip={tip}
      tipSide={tipSide}
      disabled={disabled}
    >
      {copied ? "Copied!" : !inline && "Copy"}
    </Button>
  );
}
