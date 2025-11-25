import React, { ReactNode } from "react";
import { ConvexLogo } from "@common/elements/ConvexLogo";
import { GoogleAnalytics } from "elements/GoogleAnalytics";
import { Flourish } from "components/login/Flourish";

type LayoutProps = {
  children: ReactNode;
};

export function LoginLayout({ children }: LayoutProps) {
  return (
    <div className="h-full overflow-hidden bg-background-brand">
      <GoogleAnalytics />

      <div className="flex h-full flex-col items-center">
        <div className="z-20 flex flex-1 flex-col items-center justify-center px-12">
          <div className="mb-8">
            <ConvexLogo />
          </div>
          {children}
        </div>
        <div className="h-14" />
      </div>
      <Flourish />
    </div>
  );
}
