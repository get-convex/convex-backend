import { useRouter } from "next/router";
import { useTeams } from "api/teams";
import { useEffect } from "react";
import { Loading } from "dashboard-common";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function RedirectToTeam() {
  const { selectedTeamSlug } = useTeams();
  const router = useRouter();

  useEffect(() => {
    // If we reached the catch-all route on a route starting with /t/,
    // the user tried to go to a page we don't have.
    if (router.asPath.startsWith("/t/")) {
      void router.replace("/404");
      return;
    }
    router.query.route?.length
      ? void router.replace(`/t/${router.asPath}`)
      : selectedTeamSlug && void router.replace(`/t/${selectedTeamSlug}`);
  }, [selectedTeamSlug, router]);

  return <Loading />;
}

function Main() {
  return <RedirectToTeam />;
}

export default withAuthenticatedPage(Main);
