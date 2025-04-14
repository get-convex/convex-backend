import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { ComponentsView } from "@common/features/settings/components/ComponentsView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(ComponentsView);
