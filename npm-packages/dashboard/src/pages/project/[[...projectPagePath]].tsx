import { useLastCreatedProject } from "hooks/useLastCreated";
import { useLastViewedProject } from "hooks/useLastViewed";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";
import { useEffect } from "react";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(function RedirectToProjectPage() {
  const router = useRouter();

  const { projectPagePath, ...query } = router.query;
  const path = ((projectPagePath ?? []) as string[]).join("/");

  const [lastViewedProjectSlug] = useLastViewedProject();
  const lastCreatedProject = useLastCreatedProject();
  const project = lastViewedProjectSlug ?? lastCreatedProject?.slug;

  useEffect(() => {
    void router.replace(
      project === undefined
        ? "/"
        : { pathname: `/p/${project}/${path}`, query },
    );
  });

  return null;
});
