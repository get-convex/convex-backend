import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FileStorageView } from "dashboard-common/features/files/components/FileStorageView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(FileStorageView);
