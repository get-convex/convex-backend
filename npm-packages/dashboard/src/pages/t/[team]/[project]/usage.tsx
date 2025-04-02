import { GetServerSideProps } from "next";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export const getServerSideProps: GetServerSideProps = async ({ params }) => {
  const team = params?.team;
  const project = params?.project;

  if (typeof team !== "string" || typeof project !== "string") {
    throw new Error("Invalid team or project");
  }

  return {
    redirect: {
      destination: `/t/${team}/settings/usage?projectSlug=${encodeURIComponent(project)}`,
      permanent: false,
    },
  };
};

function RedirectToTeamUsage() {
  return null;
}

export default withAuthenticatedPage(RedirectToTeamUsage);
