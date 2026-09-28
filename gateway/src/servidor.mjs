// HTTP del gateway: POST /send (worker del backend) + GET /media/* (289A-1).
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { config } from "./config.mjs";
import { purgar } from "./media.mjs";
import { listarEstados } from "./estado.mjs";
import { registrarEnviado } from "./eco.mjs";

const MIME_POR_EXT = { jpg: "image/jpeg", jpeg: "image/jpeg", png: "image/png", webp: "image/webp" };

async function leerJson(req) {
  const trozos = [];
  for await (const t of req) trozos.push(t);
  const crudo = Buffer.concat(trozos).toString("utf8");
  if (crudo.length > 256 * 1024) throw new Error("cuerpo demasiado grande");
  return JSON.parse(crudo);
}

async function enviar(sock, { destino, texto, media_url }) {
  const jid = `${(destino ?? "").replace(/\D/g, "")}@s.whatsapp.net`;
  if (media_url) {
    const res = await fetch(media_url);
    if (!res.ok) throw new Error(`media ${res.status}`);
    const buffer = Buffer.from(await res.arrayBuffer());
    await sock.sendMessage(jid, { image: buffer, caption: texto });
  } else {
    await sock.sendMessage(jid, { text: texto });
  }
}

/* Secreto worker/admin → gateway. Misma regla que el webhook del
 * backend: si no está configurado se acepta (red local de pruebas). */
function secretoValido(req) {
  return !config.sendSecreto || req.headers["x-gateway-secret"] === config.sendSecreto;
}

export function arrancarServidor(sockets) {
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url, "http://x");
      /* Panel admin (vía proxy del backend): estado + QR. El QR solo
       * existe mientras espera escaneo; vinculada devuelve 404 para no
       * mostrar un QR ya inútil. */
      if (req.method === "GET" && url.pathname === "/sesiones") {
        if (!secretoValido(req)) {
          res.writeHead(401, { "content-type": "application/json" }).end('{"error":"no autorizado"}');
          return;
        }
        res.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify(listarEstados()));
        return;
      }
      if (req.method === "GET" && url.pathname.startsWith("/sesiones/") && url.pathname.endsWith("/qr")) {
        if (!secretoValido(req)) {
          res.writeHead(401, { "content-type": "application/json" }).end('{"error":"no autorizado"}');
          return;
        }
        const via = url.pathname.split("/")[2];
        const sesion = listarEstados().find((s) => s.via === via);
        if (!sesion || sesion.estado !== "esperando_qr") {
          res.writeHead(404, { "content-type": "application/json" }).end('{"error":"sin QR pendiente"}');
          return;
        }
        try {
          const png = await readFile(new URL(`../qr-${via}.png`, import.meta.url));
          res.writeHead(200, { "content-type": "image/png", "cache-control": "no-store" }).end(png);
        } catch {
          res.writeHead(404, { "content-type": "application/json" }).end('{"error":"sin QR pendiente"}');
        }
        return;
      }
      if (req.method === "GET" && url.pathname.startsWith("/media/")) {
        const nombre = url.pathname.slice("/media/".length);
        if (!/^[\w-]+\.(jpg|jpeg|png|webp)$/.test(nombre)) {
          res.writeHead(404).end();
          return;
        }
        try {
          const datos = await readFile(new URL(nombre, config.mediaDir));
          const ext = nombre.split(".").pop();
          res.writeHead(200, { "content-type": MIME_POR_EXT[ext] }).end(datos);
        } catch {
          res.writeHead(404).end();
        }
        return;
      }
      if (req.method === "POST" && url.pathname === "/send") {
        if (!secretoValido(req)) {
          res.writeHead(401, { "content-type": "application/json" }).end('{"error":"no autorizado"}');
          return;
        }
        const cuerpo = await leerJson(req);
        const via = cuerpo.via === "wa_b" ? "wa_b" : "wa_a";
        const sock = sockets.get(via);
        if (!sock) {
          res.writeHead(503, { "content-type": "application/json" }).end('{"error":"sesion no lista"}');
          return;
        }
        const texto = (cuerpo.texto ?? "").toString();
        const destino = (cuerpo.destino ?? "").toString();
        if (!texto || texto.length > 4000 || !/^\d{7,15}$/.test(destino.replace(/\D/g, ""))) {
          res.writeHead(400, { "content-type": "application/json" }).end('{"error":"destino o texto invalido"}');
          return;
        }
        await enviar(sock, { destino, texto, media_url: cuerpo.media_url });
        /* Anti-eco 289A-1: este texto volverá como inbound en la sesión del
         * otro número; registrarlo evita re-ingestarlo como `client`. */
        registrarEnviado(destino, texto);
        res.writeHead(200, { "content-type": "application/json" }).end('{"ok":true}');
        return;
      }
      if (req.method === "GET" && url.pathname === "/health") {
        const estado = Object.fromEntries([...sockets.keys()].map((k) => [k, true]));
        res.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify({ ok: true, sesiones: estado }));
        return;
      }
      res.writeHead(404).end();
    } catch (e) {
      console.error("HTTP gateway:", e.message);
      res.writeHead(500, { "content-type": "application/json" }).end('{"error":"interno"}');
    }
  });
  server.listen(config.puerto, "127.0.0.1", () => {
    console.log(`gateway HTTP en http://127.0.0.1:${config.puerto}`);
  });
  setInterval(() => {
    purgar().then((n) => {
      if (n > 0) console.log(`media temporal: ${n} purgados`);
    });
  }, 10 * 60 * 1000).unref();
  return server;
}
