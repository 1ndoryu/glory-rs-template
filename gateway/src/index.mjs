// Entrada del gateway: sesiones A+B + HTTP /send + /media (289A-1).
import { conectarSesion } from "./sesion.mjs";
import { arrancarServidor } from "./servidor.mjs";

const sockets = new Map();
arrancarServidor(sockets);
await conectarSesion("wa_a", sockets);
await conectarSesion("wa_b", sockets);
console.log("gateway listo: escanea los QR que aparezcan arriba");
