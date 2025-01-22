import { useLastCreatedTeam } from "hooks/useLastCreated";
import { useLastViewedTeam } from "hooks/useLastViewed";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useEffect } from "react";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function RedirectToTeamPage() {
  const router = useRouter();

  const { teamPagePath, ...query } = router.query;
  const path = ((teamPagePath ?? []) as string[]).join("/");

  const [lastViewedTeamSlug] = useLastViewedTeam();
  const lastCreatedTeam = useLastCreatedTeam();
  const team = lastViewedTeamSlug ?? lastCreatedTeam?.slug;

  useEffect(() => {
    void router.replace(
      team === undefined ? "/" : { pathname: `/t/${team}/${path}`, query },
    );
  });

  return null;
});
