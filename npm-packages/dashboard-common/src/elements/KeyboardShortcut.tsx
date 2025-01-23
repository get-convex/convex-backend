import classNames from "classnames";
import { useEffect, useState } from "react";

export type Key =
  | "CtrlOrCmd" // depending on platform
  | "Ctrl"
  | "Alt"
  | "Shift"
  | "Esc"
  | "Return"
  | "Delete"
  | "Backspace"
  | "Space"
  | "Up"
  | "Right"
  | "Left"
  | "Down"
  | "Tab"
  | "A"
  | "B"
  | "C"
  | "D"
  | "E"
  | "F"
  | "G"
  | "H"
  | "I"
  | "J"
  | "K"
  | "L"
  | "M"
  | "N"
  | "O"
  | "P"
  | "Q"
  | "R"
  | "S"
  | "T"
  | "U"
  | "V"
  | "W"
  | "X"
  | "Y"
  | "Z"
  | "0"
  | "1"
  | "2"
  | "3"
  | "4"
  | "5"
  | "6"
  | "7"
  | "8"
  | "9";

type PlatformKeyNameOverrides = {
  [key in Key]?: string;
};
const appleKeyNameOverrides: PlatformKeyNameOverrides = {
  CtrlOrCmd: "⌘",
  Shift: "⇧",
  Alt: "⌥",
  Ctrl: "⌃",
  Return: "⏎",
  Esc: "esc",
  Backspace: "⌫",
  Delete: "⌦",
  Right: "→",
  Left: "←",
  Up: "↑",
  Down: "↓",
  Tab: "⇥",
};
const nonAppleKeyNameOverrides: PlatformKeyNameOverrides = {
  CtrlOrCmd: "Ctrl",
};

export function KeyboardShortcut({
  value,
  isApple = "auto",
  className,
}: {
  value: Key[];
  isApple?: boolean | "auto";
  className?: string;
}) {
  const [isAppleDetected, setIsAppleDetected] = useState<boolean | undefined>();
  useEffect(() => {
    setIsAppleDetected(
      navigator.platform.includes("Mac") ||
        navigator.platform.includes("iPhone") ||
        navigator.platform.includes("iPad"),
    );
  }, []);

  const resolvedIsApple = isApple === "auto" ? isAppleDetected : isApple;
  if (resolvedIsApple === undefined) return null;

  return (
    <kbd className={classNames("font-sans", className)}>
      {value
        .map((key) => {
          const overrides = resolvedIsApple
            ? appleKeyNameOverrides
            : nonAppleKeyNameOverrides;
          return key in overrides ? overrides[key] : key;
        })
        .join(resolvedIsApple ? "" : "+")}
    </kbd>
  );
}
