import React from "react";
import { AIButton } from "../AIButton/AIButton";
import Sparkle from "../AIButton/sparkle.svg";

export function ConvexAiChat() {
  return (
    <AIButton className="js-launch-kapa-ai h-11 order-1 mr-2 lg:order-2 lg:mr-0 lg:ml-2 shrink-0">
      <div className="flex items-center gap-2 pl-2 pr-3">
        <Sparkle className="w-5 h-5" />
        Ask AI
      </div>
    </AIButton>
  );
}
