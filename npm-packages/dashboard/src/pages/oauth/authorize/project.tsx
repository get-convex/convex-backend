import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthorizeProject } from "components/AuthorizeProject";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(AuthorizeProject);
