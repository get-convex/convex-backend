import { Toaster } from "sonner";
import { useTheme } from "next-themes";

export function ToastContainer() {
  const { resolvedTheme } = useTheme();

  return (
    <Toaster
      theme={resolvedTheme === "dark" ? "dark" : "light"}
      position="bottom-right"
      closeButton
    />
  );
}
