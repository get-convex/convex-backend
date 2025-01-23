import { ArrowRightIcon } from "@radix-ui/react-icons";
import React from "react";
import { AIButton } from "../AIButton/AIButton";
import Sparkle from "../AIButton/sparkle.svg";

declare global {
  interface Window {
    Kapa: {
      open: (config: { mode: string; query: string; submit: boolean }) => void;
    };
  }
}

interface AskAIProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  query: string;
}

export default function AskAI({ query, onClick, ...rest }: AskAIProps) {
  const promptPrefix = "Can you tell me about ";
  const promptSuffix = "?";

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    window.Kapa.open({
      mode: "ai",
      query: `${promptPrefix}${query}${promptSuffix}`,
      submit: true,
    });

    // Call the parent onClick handler.
    onClick?.(e);
  };

  return (
    <AIButton className="shrink-0 p-[5px]" onClick={handleClick} {...rest}>
      <div className="flex p-5 pl-4 items-center gap-4 w-full">
        <Sparkle className="w-8 h-8" />
        <div className="flex flex-col grow text-left text-neutral-n12 dark:text-neutral-n3">
          <span className="text-lg leading-snug">
            {promptPrefix}
            <strong className="text-neutral-black dark:text-neutral-white font-semibold">
              {query}
            </strong>
            {promptSuffix}
          </span>
          <span className="text-sm opacity-70">Get an instant AI answer</span>
        </div>
        <ArrowRightIcon className="w-6 h-6 relative -translate-x-0.5 group-hover:translate-x-0.5 transition-transform" />
      </div>
    </AIButton>
  );
}
