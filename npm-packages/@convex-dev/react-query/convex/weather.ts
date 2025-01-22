import { action } from "./_generated/server.js";

export const getSFWeather = action(async () => {
  const stationUrl = "https://api.weather.gov/stations/SFOC1";
  const observationData = await (
    await fetch(`${stationUrl}/observations/latest`, {
      headers: { "User-Agent": "Convex/1.0" },
    })
  ).json();
  const celsius = observationData.properties.temperature.value;
  return {
    fahrenheit: Math.round(((celsius * 9) / 5 + 32) * 100) / 100,
    fetchedAt: Date.now(),
    timestamp: observationData.properties.timestamp,
  };
});
