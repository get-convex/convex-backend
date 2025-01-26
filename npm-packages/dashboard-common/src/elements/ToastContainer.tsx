import { useTheme } from "dashboard-common";
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
