import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthorizeTeam } from "components/AuthorizeTeam";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(AuthorizeTeam);
