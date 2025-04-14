import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FileStorageView } from "@common/features/files/components/FileStorageView";

export { getServerSideProps } from "lib/ssr";

export default withAuthenticatedPage(FileStorageView);
