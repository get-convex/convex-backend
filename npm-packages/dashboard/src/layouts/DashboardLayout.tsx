import React, { ReactNode } from "react";
import { GoogleAnalytics } from "elements/GoogleAnalytics";

type LayoutProps = {
  children: ReactNode;
};

export function DashboardLayout({ children }: LayoutProps) {
  return (
    <>
      <GoogleAnalytics />
      <div className="scrollbar flex h-screen flex-col overflow-y-auto">
        {children}
      </div>
    </>
  );
}
