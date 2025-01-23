import React from "react";

export function Breadcrumbs({ children }: { children: React.ReactNode[] }) {
  const filteredChildren = children.filter((child) => child !== null);
  return (
    <div className="flex items-center gap-2">
      {filteredChildren.map((child, index) => (
        <React.Fragment key={index}>
          {child}
          {index !== filteredChildren.length - 1 && (
            <span className="text-content-secondary" role="separator">
              /
            </span>
          )}
        </React.Fragment>
      ))}
    </div>
  );
}
