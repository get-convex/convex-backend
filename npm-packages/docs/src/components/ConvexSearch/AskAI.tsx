import { ArrowRightIcon } from "@radix-ui/react-icons";
import React from "react";
import { AIButton } from "../AIButton/AIButton";
import Sparkle from "../AIButton/sparkle.svg";

interface AskAIProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {}

export default function AskAI(props: AskAIProps) {
  return (
    <AIButton className="js-launch-kapa-ai shrink-0 p-[5px]" {...props}>
      <div className="flex p-5 pl-4 items-center gap-4 w-full">
        <Sparkle className="w-8 h-8" />
        <div className="flex flex-col grow text-left">
          <strong className="text-lg leading-snug">Ask AI</strong>
          <span className="text-sm opacity-80">Get an instant AI answer</span>
        </div>
        <ArrowRightIcon className="w-6 h-6 relative -translate-x-0.5 group-hover:translate-x-0.5 transition-transform" />
      </div>
    </AIButton>
  );
}
