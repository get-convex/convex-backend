import { useRouter } from "next/router";
import { useTeams } from "api/teams";
import { useEffect } from "react";
import { Loading } from "@ui/Loading";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useProjectById } from "api/projects";
import { useSupportFormOpen } from "elements/SupportWidget";

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
  const { query, push } = useRouter();
  const { teams } = useTeams();
  const [, setOpenState] = useSupportFormOpen();

  // If vercelPath is set, we need to redirect somewhere, but we don't know
  // which team to redirect to. We'll redirect to the first team that is managed
  // by Vercel. In most cases, this will be correct.
  const firstVercelTeam = teams?.find((t) => t.managedBy === "vercel");
  if (query.vercelPath === "support") {
    setOpenState(true);
    void push(firstVercelTeam ? `/t/${firstVercelTeam.slug}` : "/");
    return <Loading />;
  }

  if (query.vercelPath === "billing") {
    void push(
      firstVercelTeam
        ? `/t/${firstVercelTeam.slug}/settings/billing`
        : "/team/settings/billing",
    );
    return <Loading />;
  }
  if (query.vercelPath === "usage") {
    void push(
      firstVercelTeam
        ? `/t/${firstVercelTeam.slug}/settings/usage`
        : "/team/settings/usage",
    );
    return <Loading />;
  }
  if (query.invoiceId) {
    // TODO(ENG-9453): Support this.
  }

  if (query.projectId) {
    return <RedirectToProjectById id={query.vercel_resource_id as string} />;
  }
  return <RedirectToTeam />;
}

function RedirectToProjectById({ id }: { id: string }) {
  const project = useProjectById(parseInt(id));
  const { teams } = useTeams();
  const projectTeam = teams?.find((team) => team.id === project?.teamId);
  const router = useRouter();
  useEffect(() => {
    if (project && projectTeam) {
      void router.replace(`/t/${projectTeam.slug}/${project.slug}`);
    }
  }, [project, projectTeam, router]);
  return <Loading />;
}

export default withAuthenticatedPage(Main);
