import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthorizeApp } from "components/AuthorizeApp";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(() => (
  <AuthorizeApp authorizationScope="project" />
));
