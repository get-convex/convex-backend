import { Loading } from "@ui/Loading";
import { toast } from "@common/lib/utils";
import { useDiscordAuthorize } from "api/discord";
import { useAuthHeader } from "hooks/fetching";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";

export { getServerSideProps } from "lib/ssr";

function Discord() {
  const authHeader = useAuthHeader();
  const router = useRouter();
  const discordAuthorize = useDiscordAuthorize();

  const [hasRun, setHasRun] = useState(false);

  useEffect(() => {
    if (!authHeader || router.query === undefined || hasRun) return;
    setHasRun(true);

    const { code, state } = router.query;
    const isAuthorizing = typeof code === "string" && typeof state === "string";

    void (async () => {
      if (isAuthorizing) {
        try {
          await discordAuthorize({ authorizationCode: code, csrfToken: state });
          await router.push(
            "https://discord.com/channels/1019350475847499849/1019350478817079338",
          );
        } catch (e) {
          await router.push("/");
        }
      } else {
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/discord/login_url`,
          {
            headers: {
              Authorization: authHeader,
              "Content-Type": "application/json",
            },
          },
        );

        if (response.ok) {
          const { url } = await response.json();
          await router.push(url);
        } else {
          const { message } = await response.json();
          toast("error", message);
          await router.push("/");
        }
      }
    })();
  }, [authHeader, discordAuthorize, hasRun, router]);

  return (
    <div className="h-screen">
      <Loading />
    </div>
  );
}

export default withAuthenticatedPage(Discord);
