import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

// We never hit this page, because we redirect in SSR to the appropriate
// project page
export default withAuthenticatedPage(function ProjectPage() {
  return null;
});
