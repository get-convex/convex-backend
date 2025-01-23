import { useMemo } from "react";
import { useSessionStorage } from "react-use";

export function useAuthorizeProdEdits({ isProd }: { isProd: boolean }) {
  const [prodEditsEnabled, setProdEditsEnabled] = useSessionStorage(
    "prodEditsEnabled",
    false,
  );
  const onAuthorizeEdits = useMemo(
    () => (isProd ? () => setProdEditsEnabled(true) : undefined),
    [isProd, setProdEditsEnabled],
  );
  const areEditsAuthorized = !isProd || prodEditsEnabled;
  return [areEditsAuthorized, onAuthorizeEdits] as const;
}
