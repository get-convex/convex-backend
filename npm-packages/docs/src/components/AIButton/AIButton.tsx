import { cn } from "@site/src/lib/cn";
import React from "react";

interface AIButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {}

export function AIButton({ children, className, ...props }: AIButtonProps) {
  return (
    <button
      className={cn(
        "relative bg-transparent p-[3px] border-none overflow-hidden rounded-lg bg-gradient-to-r from-plum-p4/90 via-red-r3/90 to-yellow-y3/90 cursor-pointer group hover:from-plum-p4 hover:via-red-r3 hover:to-yellow-y3 transition-colors",
        className,
      )}
      {...props}
    >
      <div className="bg-neutral-white/90 rounded-[0.3rem] h-full flex font-sans text-sm text-neutral-n10 font-medium group-hover:bg-neutral-white group-hover:text-neutral-n12 transition-colors dark:bg-neutral-black/90 dark:text-neutral-n4 dark:group-hover:bg-neutral-black dark:group-hover:text-neutral-n2">
        {children}
      </div>
    </button>
  );
}
