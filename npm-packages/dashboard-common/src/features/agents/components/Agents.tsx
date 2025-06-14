import { Nent } from "@common/lib/useNents";

export function Agents({ nents: allNents }: { nents: Nent[] }) {
  const nents = allNents.filter((nent) => nent.name !== null);
  const usingAgentComponent = nents.some(
    (nent) => nent.name === "agent" && nent.path === "agent"
  );
  if (usingAgentComponent) {
    return <div>Using AI Agents... show dashboard</div>;
  }
  return <div>Not using agents... follow docs to get set up</div>;
}
