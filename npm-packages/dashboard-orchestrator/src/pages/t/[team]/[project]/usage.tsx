// Project-level usage redirects to the team usage page with the project slug
// in the query string — same pattern as cloud's
// dashboard/src/pages/t/[team]/[project]/usage.tsx so deep-links from the
// Project Settings sidebar land in a familiar place.

import { useRouter } from "next/router";
import { useEffect } from "react";

export default function ProjectUsageRedirect() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  useEffect(() => {
    if (!teamSlug || !projectSlug) return;
    void router.replace(
      `/t/${teamSlug}/settings/usage?projectSlug=${encodeURIComponent(
        projectSlug,
      )}`,
    );
  }, [teamSlug, projectSlug, router]);
  return null;
}
