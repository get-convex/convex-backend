import { useRouter } from "next/router";
import { useEffect } from "react";

export default function PauseDeploymentRedirect() {
  const router = useRouter();

  useEffect(() => {
    void router.replace("/settings#pause-deployment");
  }, [router]);

  return null;
}
