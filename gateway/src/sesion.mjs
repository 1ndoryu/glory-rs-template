// Sesión Baileys: conexión, QR, inbound → webhook (289A-1).
import { rm } from "node:fs/promises";
import makeWASocket, {
  useMultiFileAuthState,
  DisconnectReason,
  downloadMediaMessage,
  getContentType,
} from "@whiskeysockets/baileys";
import qrcode from "qrcode-terminal";
import QRCode from "qrcode";
import { config } from "./config.mjs";
import { avisarBackend } from "./backend.mjs";
import { guardarTemporal } from "./media.mjs";
import { setEstado } from "./estado.mjs";

const soloDigitos = (s) => (s ?? "").replace(/\D/g, "");
const jidANumero = (jid) => soloDigitos((jid ?? "").split("@")[0].split(":")[0]);

function textoDe(mensaje) {
  const m = mensaje.message;
  if (!m) return "";
  return (
    m.conversation ||
    m.extendedTextMessage?.text ||
    m.imageMessage?.caption ||
    m.videoMessage?.caption ||
    ""
  ).trim();
}

async function aPayload(msg, numeroDestino) {
  const tipo = getContentType(msg.message);
  let mediaUrl;
  let texto = textoDe(msg);
  if (tipo === "imageMessage") {
    try {
      const buffer = await downloadMediaMessage(msg, "buffer", {});
      mediaUrl = await guardarTemporal(buffer, "image/jpeg");
    } catch (e) {
      console.error(`[${numeroDestino}] no se pudo bajar foto:`, e.message);
    }
    if (!texto) texto = "(foto sin pie)";
  }
  if (!texto) return null;
  return {
    numero_destino: numeroDestino,
    remitente: jidANumero(msg.key.remoteJid),
    texto,
    ...(msg.pushName ? { nombre: msg.pushName } : {}),
    ...(mediaUrl ? { media_url: mediaUrl } : {}),
  };
}

export async function conectarSesion(via, sockets) {
  const sesion = config.sesiones[via];
  const dirAuth = new URL(`../sesiones/${via}/`, import.meta.url);
  const { state, saveCreds } = await useMultiFileAuthState(dirAuth);
  const sock = makeWASocket({
    auth: state,
    browser: ["MN Gateway", "Chrome", "1.0"],
  });
  sockets.set(via, sock);
  sock.ev.on("creds.update", saveCreds);

  sock.ev.on("connection.update", async ({ connection, lastDisconnect, qr }) => {
    if (qr) {
      setEstado(via, "esperando_qr");
      console.log(`\n=== QR sesión ${sesion.nombre} (${sesion.numero}): escanéalo con ese número ===`);
      qrcode.generate(qr, { small: true });
      /* PNG para escanear desde el explorador (el QR caduca en ~1 min;
       * si vence, el gateway genera otro y sobrescribe el archivo). */
      const png = new URL(`../qr-${via}.png`, import.meta.url);
      await QRCode.toFile(png, qr, { width: 512 }).catch((e) => {
        console.error(`[${sesion.nombre}] no se pudo escribir QR png:`, e.message);
      });
    }
    if (connection === "open") {
      setEstado(via, "abierta");
      console.log(`[${sesion.nombre}] sesión abierta`);
    }
    if (connection === "close") {
      setEstado(via, "cerrada");
      const codigo = lastDisconnect?.error?.output?.statusCode;
      if (codigo === DisconnectReason.loggedOut) {
        console.log(`[${sesion.nombre}] desvinculado: borro auth, re-escanea para reconectar`);
        await rm(dirAuth, { recursive: true, force: true });
        setTimeout(() => conectarSesion(via, sockets), 5000);
      } else {
        console.log(`[${sesion.nombre}] conexión cerrada (${codigo}), reconecto en 5s`);
        setTimeout(() => conectarSesion(via, sockets), 5000);
      }
    }
  });

  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    if (type !== "notify") return;
    for (const msg of messages) {
      if (msg.key.fromMe || !msg.message) continue;
      try {
        const payload = await aPayload(msg, sesion.numero);
        if (!payload) continue;
        await avisarBackend(payload);
        console.log(`[${sesion.nombre}] → webhook: ${payload.remitente} (${payload.texto.length} chars)`);
      } catch (e) {
        console.error(`[${sesion.nombre}] inbound falló:`, e.message);
      }
    }
  });

  return sock;
}
