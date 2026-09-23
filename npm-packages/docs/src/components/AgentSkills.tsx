import skills from "../data/agent-skills.json";

const SKILLS_BASE_URL =
  "https://github.com/get-convex/agent-skills/blob/main/skills";

export function AgentSkills() {
  return (
    <table>
      <thead>
        <tr>
          <th>Skill</th>
          <th>Description</th>
        </tr>
      </thead>
      <tbody>
        {skills.skills.map(({ name, description }) => (
          <tr key={name}>
            <td>
              <a href={`${SKILLS_BASE_URL}/${name}/SKILL.md`}>
                <code>/{name}</code>
              </a>
            </td>
            <td>{description}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
