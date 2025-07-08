import classNames from "classnames";
import { forwardRef } from "react";

type SheetProps = {
  className?: string;
  padding?: boolean;
  children: React.ReactNode;
} & React.HTMLAttributes<HTMLDivElement>;
export const Sheet = forwardRef<HTMLDivElement, SheetProps>(function Sheet(
  { children, className, padding = true, ...props },
  ref,
) {
  return (
    <div
      className={classNames(
        "bg-background-secondary rounded-sm border text-sm",
        padding && "p-6",
        className,
      )}
      ref={ref}
      {...props}
    >
      {children}
    </div>
  );
});
