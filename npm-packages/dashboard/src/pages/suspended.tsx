import { LoginLayout } from "layouts/LoginLayout";

export default function Suspended() {
  return (
    <LoginLayout>
      <div className="flex gap-2 divide-x text-content-primary">
        <h2>Suspended</h2>
        <div className="flex items-center gap-1 pl-2">
          <p>Team is suspended.</p>
        </div>
      </div>
    </LoginLayout>
  );
}
