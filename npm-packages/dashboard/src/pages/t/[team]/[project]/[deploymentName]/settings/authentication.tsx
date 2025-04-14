import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { AuthenticationView } from "@common/features/settings/components/AuthenticationView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(AuthenticationView);
