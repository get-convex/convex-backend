import React, { ReactNode } from "react";
import { ConvexLogo } from "dashboard-common";
import { GoogleAnalytics } from "elements/GoogleAnalytics";

import FlourishTop from "components/login/images/flourish-top.svg";
import FlourishBottom from "components/login/images/flourish-bottom.svg";
import FlourishBottomRight from "components/login/images/flourish-bottom-right.svg";
import FlourishRight from "components/login/images/flourish-right.svg";
import FlourishLeft from "components/login/images/flourish-left.svg";
import { useWindowSize } from "react-use";

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

export function Flourish() {
  const { height } = useWindowSize();
  return height > 500 ? (
    <div className="hidden dark:hidden md:block">
      <div className="absolute left-1/2 top-0 -translate-x-1/2 translate-y-[-20%]">
        <FlourishTop />
      </div>
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2">
        <FlourishBottom />
      </div>
      <div className="absolute bottom-[35%] right-0">
        <FlourishRight />
      </div>
      <div className="absolute bottom-[20%] left-0 -translate-y-1/2">
        <FlourishLeft />
      </div>
      <div className="absolute bottom-0 right-[8%]">
        <FlourishBottomRight />
      </div>
    </div>
  ) : null;
}
