// Estado de sesiones para el panel admin (289A-1): `iniciando` →
// `esperando_qr` → `abierta` (`cerrada` al caer). Lo escribe `sesion.mjs`,
// lo lee `servidor.mjs` (`GET /sesiones`).
import { config } from "./config.mjs";

const estados = new Map();
for (const via of Object.keys(config.sesiones)) {
  estados.set(via, { estado: "iniciando" });
}

export function setEstado(via, estado) {
  const e = estados.get(via);
  if (e) e.estado = estado;
}

export function listarEstados() {
  return [...estados.entries()].map(([via, e]) => ({
    via,
    nombre: config.sesiones[via].nombre,
    numero: config.sesiones[via].numero,
    estado: e.estado,
  }));
}
