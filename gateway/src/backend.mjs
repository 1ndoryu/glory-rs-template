// POST al webhook del backend con el secreto compartido (289A-1).
import { config } from "./config.mjs";

export async function avisarBackend(payload) {
  const res = await fetch(config.backendWebhook, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(config.webhookSecreto ? { "X-Gateway-Secret": config.webhookSecreto } : {}),
    },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    const cuerpo = await res.text().catch(() => "");
    throw new Error(`webhook ${res.status}: ${cuerpo.slice(0, 200)}`);
  }
  return res.json();
}
