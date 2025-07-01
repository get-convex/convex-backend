import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { IntegrationsView } from "@common/features/settings/components/integrations/IntegrationsView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(IntegrationsView);
