import { ReactNode } from "react";
import Link from "next/link";
import classNames from "classnames";
import { UrlObject } from "url";
import { DotsVerticalIcon } from "@radix-ui/react-icons";
import { Menu, MenuItem } from "@ui/Menu";

export type CardProps = {
  cardClassName?: string;
  contentClassName?: string;
  href?: string | UrlObject;
  children: ReactNode;
  dropdownItems?: {
    Icon: React.FC<{ className: string | undefined }>;
    text: string;
    action: () => void;
    destructive?: boolean;
    disabled?: boolean;
    tip?: string;
  }[];
  // Will be rendered as sibling to Card content
  overlayed?: ReactNode;
};

export function Card({
  cardClassName,
  contentClassName,
  href,
  children,
  dropdownItems,
  overlayed,
}: CardProps) {
  return (
    <div
      className={classNames(
        "relative border rounded-xl bg-background-secondary",
        "flex items-center gap-4 px-4",
        "hover:border-border-selected",
        cardClassName,
      )}
    >
      {!href ? (
        <div className={classNames("grow py-4", contentClassName)}>
          {children}
        </div>
      ) : (
        <Link
          href={href}
          passHref
          className={classNames(
            "grow cursor-pointer min-w-0 py-4",
            contentClassName,
          )}
        >
          {children}
        </Link>
      )}

      {overlayed}
      {dropdownItems && (
        <Menu
          placement="bottom-start"
          buttonProps={{
            "aria-label": "Open project settings",
            variant: "neutral",
            inline: true,
            icon: <DotsVerticalIcon className="text-content-secondary" />,
          }}
        >
          {dropdownItems?.map((item) => (
            <MenuItem
              key={item.text}
              variant={item.destructive ? "danger" : "default"}
              action={item.action}
              disabled={item.disabled}
              tip={item.tip}
              tipSide="right"
            >
              <item.Icon className="h-4 w-4" />
              {item.text}
            </MenuItem>
          ))}
        </Menu>
      )}
    </div>
  );
}
