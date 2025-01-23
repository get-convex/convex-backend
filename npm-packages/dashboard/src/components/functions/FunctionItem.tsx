import React from "react";
import {
  FunctionIcon,
  sidebarLinkClassNames,
  Button,
  Tooltip,
  ModuleFunction,
  useCurrentOpenFunction,
  useLogDeploymentEvent,
  useCurrentGloballyOpenFunction,
} from "dashboard-common";
import { LockClosedIcon } from "@radix-ui/react-icons";
import { useRouter } from "next/router";
import { cn } from "lib/cn";
import { Disclosure } from "@headlessui/react";

export function FunctionItem({
  item,
  showFileName,
  onChangeFunction,
  nestingLevel,
}: {
  item: ModuleFunction;
  showFileName?: boolean;
  onChangeFunction: () => void;
  nestingLevel: number;
}) {
  const router = useRouter();
  const currentOpenFunction = useCurrentOpenFunction();
  const log = useLogDeploymentEvent();
  const [, setGloballyOpenFunction] = useCurrentGloballyOpenFunction();
  const isCurrentFunction = currentOpenFunction?.identifier === item.identifier;
  return (
    <DirectoryItem
      href={{ query: { ...router.query, function: item.displayName } }}
      onClick={() => {
        onChangeFunction();
        setGloballyOpenFunction(item);
        log("change function");
      }}
      isActive={isCurrentFunction}
      nestingLevel={nestingLevel}
      key={item.name}
    >
      <FunctionIcon className="size-4 shrink-0 text-content-tertiary" />
      <div className="grow truncate">
        {/* If set to show the full file name, and the file name is not the default export, concatenate the file name with the function name */}
        {showFileName &&
        item.file.name !== item.name &&
        // HTTP actions have unique names
        item.udfType !== "HttpAction"
          ? `${item.file.name}:${item.name}`
          : item.name}
      </div>
      {item.visibility.kind !== "public" && (
        <Tooltip tip="This is an internal function." side="right">
          <LockClosedIcon className="size-3 shrink-0 text-content-tertiary" />
        </Tooltip>
      )}
    </DirectoryItem>
  );
}

function paddingForLevel(level: number) {
  if (level >= 1) {
    return 28;
  }
  return 12;
}

export function DirectoryItem({
  href,
  onClick,
  isActive = false,
  nestingLevel,
  children,
  disclosure = false,
}: {
  href?: { query: { [key: string]: string } };
  onClick?: () => void;
  isActive?: boolean;
  nestingLevel: number;
  children: React.ReactNode[];
  disclosure?: boolean;
}) {
  const Btn = disclosure ? Disclosure.Button : Button;
  return (
    <Btn
      variant="unstyled"
      className={cn(
        sidebarLinkClassNames({
          isActive,
          font: "mono",
          small: true,
        }),
        "px-0 py-0 w-full min-w-full max-w-full truncate h-[30px] pr-2",
        "rounded-none",
        isActive &&
          "outline outline-util-accent/40 bg-util-accent/30 hover:bg-util-accent/30 font-normal",
        !isActive && "hover:bg-util-accent/20",
        "focus-visible:outline-none focus-visible:ring-0 focus-visible:bg-util-accent/20",
      )}
      href={href}
      onClick={onClick}
    >
      <div
        className="flex h-full items-center gap-4"
        style={{
          paddingLeft: `${paddingForLevel(nestingLevel)}px`,
        }}
      >
        {nestingLevel !== 0 &&
          Array.from({ length: nestingLevel }).map((_, index) => (
            <div
              key={index}
              className="h-full w-[1px] shrink-0 bg-border-transparent"
            />
          ))}
      </div>
      {children}
    </Btn>
  );
}
