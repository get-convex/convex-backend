import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DataView } from "@common/features/data/components/DataView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(DataView);
