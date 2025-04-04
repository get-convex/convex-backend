import { ReactNode } from "react";

export function PageContent({ children }: { children: ReactNode }) {
  return (
    <div className="relative h-full min-w-0 flex-1 overflow-x-auto bg-background-primary">
      {children}
    </div>
  );
}
