import React, { ReactNode } from "react";
import { GoogleAnalytics } from "elements/GoogleAnalytics";

type LayoutProps = {
  children: ReactNode;
};

export function DashboardLayout({ children }: LayoutProps) {
  return (
    <>
      <GoogleAnalytics />
      <div className="flex h-screen flex-col overflow-y-auto scrollbar">
        {children}
      </div>
    </>
  );
}
