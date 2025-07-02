import { useEffect } from "react";
import { useLocalStorage } from "react-use";

export function useLastViewedTeam() {
  return useLocalStorage<string>(`/lastViewedTeam`);
}

export function useRememberLastViewedTeam(slug: string | undefined) {
  const [, setLastViewedTeam] = useLastViewedTeam();
  useEffect(() => {
    if (slug !== undefined) {
      setLastViewedTeam(slug);
    }
  }, [slug, setLastViewedTeam]);
}

export function useLastViewedProject() {
  return useLocalStorage<string>(`/lastViewedProject`);
}

export function useRememberLastViewedProject(slug: string | undefined) {
  const [, setLastViewedProject] = useLastViewedProject();
  useEffect(() => {
    if (slug !== undefined) {
      setLastViewedProject(slug);
    }
  }, [slug, setLastViewedProject]);
}

export function useLastViewedDeployment() {
  return useLocalStorage<string>(`/lastViewedDeployment`);
}

export function useLastViewedDeploymentForProject(projectSlug: string) {
  return useLocalStorage<string>(
    `/lastViewedDeploymentForProject/${projectSlug}`,
  );
}

export function useRememberLastViewedDeploymentForProject(
  projectSlug: string,
  name: string | undefined,
) {
  const [, setLastViewedDeploymentForProject] =
    useLastViewedDeploymentForProject(projectSlug);
  useRememberLastViewedDeployment(name);
  useEffect(() => {
    if (name !== undefined) {
      setLastViewedDeploymentForProject(name);
    }
  }, [name, setLastViewedDeploymentForProject]);
}

function useRememberLastViewedDeployment(name: string | undefined) {
  const [, setLastViewedDeployment] = useLastViewedDeployment();
  useEffect(() => {
    if (name !== undefined) {
      setLastViewedDeployment(name);
    }
  }, [name, setLastViewedDeployment]);
}
