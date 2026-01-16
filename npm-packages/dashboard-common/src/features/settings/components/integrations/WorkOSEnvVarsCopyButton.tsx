import { Button } from "@ui/Button";
import { toast } from "@common/lib/utils";

interface WorkOSEnvVarsCopyButtonProps {
  clientId: string;
  apiKey?: string;
  label?: string;
  size?: "xs" | "sm" | "md";
  variant?: "primary" | "neutral" | "danger";
  tip?: string;
}

export function WorkOSEnvVarsCopyButton({
  clientId,
  apiKey,
  label = "Copy Environment Variables",
  size = "sm",
  variant = "neutral",
  tip = "Copy environment variables for your build environment",
}: WorkOSEnvVarsCopyButtonProps) {
  const envVarsText = [
    `WORKOS_CLIENT_ID="${clientId}"`,
    apiKey && `WORKOS_API_KEY="${apiKey}"`,
  ]
    .filter(Boolean)
    .join("\n");

  const handleCopy = () => {
    void navigator.clipboard.writeText(envVarsText);
    toast("success", "Environment variables copied to clipboard");
  };

  return (
    <Button onClick={handleCopy} size={size} variant={variant} tip={tip}>
      {label}
    </Button>
  );
}
