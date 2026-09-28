// Config del gateway Baileys (289A-1). Todo por entorno, sin secretos en disco.
const num = (v, def) => {
  const n = Number.parseInt(v ?? "", 10);
  return Number.isFinite(n) ? n : def;
};

export const config = {
  puerto: num(process.env.GATEWAY_PORT, 3102),
  backendWebhook: process.env.BACKEND_WEBHOOK_URL || "http://127.0.0.1:3000/api/agent/whatsapp/webhook",
  webhookSecreto: process.env.WA_WEBHOOK_SECRETO || "",
  sendSecreto: process.env.GATEWAY_SEND_SECRET || "",
  sesiones: {
    wa_a: { nombre: "A", numero: (process.env.SESSION_A_NUMBER || "584120825234").replace(/\D/g, "") },
    wa_b: { nombre: "B", numero: (process.env.SESSION_B_NUMBER || "18149575416").replace(/\D/g, "") },
  },
  mediaDir: new URL("../media/", import.meta.url),
  mediaTtlMs: 30 * 60 * 1000,
};
