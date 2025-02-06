import { useCallback, useEffect } from "react";
import { useTheme } from "next-themes";

export function ThemeConsumer() {
  const { setTheme, theme } = useTheme();

  const handleKeyPress = useCallback(
    (event: KeyboardEvent) => {
      if (event.key === "t" && event.ctrlKey && event.metaKey) {
        setTheme(theme === "dark" ? "light" : "dark");
      }
    },
    [setTheme, theme],
  );
  useEffect(() => {
    document.addEventListener("keydown", handleKeyPress);

    return () => {
      document.removeEventListener("keydown", handleKeyPress);
    };
  }, [handleKeyPress]);

  return null;
}
