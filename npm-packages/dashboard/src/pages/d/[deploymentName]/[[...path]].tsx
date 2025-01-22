import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

// Note: This route is used by the CLI!

// We never hit this page, because we redirect in SSR to the appropriate
// deployment page
export default withAuthenticatedPage(function DeploymentPage() {
  return null;
});
