// Config del gateway Baileys (289A-1). Todo por entorno, sin secretos en disco.
const num = (v, def) => {
  const n = Number.parseInt(v ?? "", 10);
  return Number.isFinite(n) ? n : def;
};

const flag = (v, def) => {
  const s = (v ?? String(def)).trim().toLowerCase();
  return !(s === "0" || s === "no" || s === "false");
};

export const config = {
  puerto: num(process.env.GATEWAY_PORT, 3102),
  backendWebhook: process.env.BACKEND_WEBHOOK_URL || "http://127.0.0.1:3110/api/agent/whatsapp/webhook", // [03AA-1] local en 3110: el 3000 lo ocupa glory-pulse.
  webhookSecreto: process.env.WA_WEBHOOK_SECRETO || "",
  sendSecreto: process.env.GATEWAY_SEND_SECRET || "",
  sesiones: {
    /* [289A-1] `respondeIA`: solo las vías en true reenvían inbound al
     * webhook. B es el teléfono manual de la dueña: mudo por defecto
     * (SESSION_B_RESPONDE_IA=1 lo reactiva sin tocar código). */
    wa_a: { nombre: "A", numero: (process.env.SESSION_A_NUMBER || "584120825234").replace(/\D/g, ""), respondeIA: flag(process.env.SESSION_A_RESPONDE_IA, 1) },
    wa_b: { nombre: "B", numero: (process.env.SESSION_B_NUMBER || "18149575416").replace(/\D/g, ""), respondeIA: flag(process.env.SESSION_B_RESPONDE_IA, 0) },
  },
  mediaDir: new URL("../media/", import.meta.url),
  mediaTtlMs: 30 * 60 * 1000,
};
