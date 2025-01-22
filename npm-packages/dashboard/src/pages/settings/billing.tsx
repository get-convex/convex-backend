import { useRouter } from "next/router";
import { useEffect } from "react";
import { Loading } from "dashboard-common";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useCurrentTeam } from "api/teams";

export { getServerSideProps } from "lib/ssr";

function RedirectToBilling() {
  const team = useCurrentTeam();
  const router = useRouter();
  useEffect(() => {
    team?.slug && void router.push(`/t/${team?.slug}/settings/billing`);
  }, [team?.slug, router]);

  return <Loading />;
}

function Main() {
  return <RedirectToBilling />;
}

export default withAuthenticatedPage(Main);
