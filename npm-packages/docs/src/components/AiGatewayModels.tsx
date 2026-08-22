import Heading from "@theme/Heading";
import catalog from "../data/ai-gateway-models.json";

export function AiGatewayModels() {
  return (
    <>
      {catalog.providers.map((provider) => (
        <section key={provider.name}>
          <Heading as="h2">{provider.name}</Heading>
          <ul>
            {provider.models.map((id) => (
              <li key={id}>
                <code>{id}</code>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}
