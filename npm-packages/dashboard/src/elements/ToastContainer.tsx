import { useTheme } from "next-themes";
import { Toaster } from "sonner";

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
