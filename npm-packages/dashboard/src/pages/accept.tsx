import { LoginTerms } from "components/login/LoginTerms";
import { LoginLayout } from "layouts/LoginLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";

export { getServerSideProps } from "lib/ssr";

function Login() {
  return (
    <LoginLayout>
      <LoginTerms />
    </LoginLayout>
  );
}

export default withAuthenticatedPage(Login);
