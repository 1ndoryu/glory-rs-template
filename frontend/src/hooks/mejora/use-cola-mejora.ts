import { useCallback, useEffect, useRef, useState } from 'react';
import type { ConfigMejora, FotoMejora, ParcheFoto } from '../../domain/foto-mejora';
import type { TipoEventoMejora } from '../../domain/historial-mejora';
import { enviarFoto, leerTrabajo, reintentarTrabajo, cancelarTrabajo } from '../../data/mejora/cliente-mejora';
import { subirMejorada } from '../../data/inmuebles/api';
import { repetirCada } from '../../platform/ventana';

/* Info de reintento backend para el badge de la tarjeta: intentos que lleva
 * el trabajo, segundos hasta el próximo reintento y último motivo. */
export interface InfoReintento {
  intentos: number;
  enSeg: number | null;
  motivo: string | null;
}

/* Seguimiento máximo de un trabajo (45 min) y reenvíos tras reinicio. */
const LIMITE_SEGUIMIENTO_MS = 45 * 60 * 1000;
const MAX_REENVIOS = 3;

// Orquesta el envío secuencial al backend local (F19: reintentos reales):
// - Una sola foto en vuelo (concurrencia 1, anti-bloqueo y anti-huérfanas:
//   si hay vuelo en curso no se marca nada sin seguidor).
// - El backend reintenta fallos temporales solo (backoff 1-2-4-8-16m, tope 5);
//   el frontal sigue el trabajo sin sentenciarlo a error.
// - Si el backend se reinició (trabajo desconocido), se re-envía el original
//   solo (máx 3) en vez de quedarse en error.
// - Si el backend no contesta al enviar, la foto queda pendiente con backoff
//   local: el modo automático la retoma solo; nada queda en error permanente
//   por una caída temporal.
// - Cada transición deja entrada en el historial visible.
// - El `jobId` se persiste en la foto: si la pestaña se cierra o recarga a
//   mitad, al volver se retoma el seguimiento en vez de quedarse colgada.
export function useColaMejora(
  fotos: FotoMejora[],
  config: ConfigMejora,
  marcar: (foto: FotoMejora, parche: ParcheFoto) => Promise<FotoMejora>,
  registrar: (foto: FotoMejora, tipo: TipoEventoMejora, detalle: string | null) => void,
) {
  const [ocupado, setOcupado] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [reintentos, setReintentos] = useState<Record<string, InfoReintento>>({});
  const enVuelo = useRef(false);
  const reconciliado = useRef(false);
  /* Backoff local por foto (fotoId -> {fallos, permitido}): evita acribillar
   * a POST un backend caído; el automático retoma al vencer. */
  const esperaLocalRef = useRef(new Map<string, { fallos: number; permitido: number }>());
  /* F21: fotos que el usuario canceló (el seguimiento en curso se retira y
   * el automático no las toca) hasta que pulse Reintentar. */
  const pausadosRef = useRef(new Set<string>());
  const canceladosRef = useRef(new Set<string>());
  const marcarRef = useRef(marcar);
  marcarRef.current = marcar;
  const registrarRef = useRef(registrar);
  registrarRef.current = registrar;
  const configRef = useRef(config);
  configRef.current = config;
  const fotosRef = useRef(fotos);
  fotosRef.current = fotos;

  const olvidarReintento = useCallback((fotoId: string) => {
    setReintentos((m) => {
      if (!(fotoId in m)) return m;
      const copia = { ...m };
      delete copia[fotoId];
      return copia;
    });
  }, []);

  // Sigue un trabajo ya encolado hasta lista o error FINAL. Racha de 12
  // nulos seguidos (1 min sin contacto) = backend caído: error con motivo.
  // Los estados no terminales (encolado con reintento programado, procesando)
  // no sentencian: se espera lo que pida el backend. Si el trabajo es
  // desconocido (backend reiniciado y cola perdida), se re-envía solo.
  const seguirTrabajo = useCallback(
    async (foto: FotoMejora, jobIdInicial: string, prompt: string): Promise<boolean> => {
      if (enVuelo.current) return false;
      enVuelo.current = true;
      setOcupado(true);
      try {
        let jobId = jobIdInicial;
        let nulos = 0;
        let reenvios = 0;
        let ultimoIntentosVistos = -1;
        let pausaSeg = 5;
        const inicio = Date.now();
        for (;;) {
          if (Date.now() - inicio > LIMITE_SEGUIMIENTO_MS) {
            await marcarRef.current(foto, {
              estado: 'error',
              error: 'Se superó el tiempo máximo de seguimiento (45 min). Reinténtalo.',
            });
            registrarRef.current(foto, 'error', 'Seguimiento más allá de 45 min.');
            olvidarReintento(foto.id);
            return false;
          }
          await new Promise((r) => setTimeout(r, pausaSeg * 1000));
          // Cancelado por el usuario mientras se esperaba: fuera del bucle
          // (el sondeo confirmará el estado y la dejará pendiente).
          if (canceladosRef.current.has(foto.id)) return false;
          const estado = await leerTrabajo(jobId);
          if (!estado) {
            nulos += 1;
            if (nulos >= 12) {
              await marcarRef.current(foto, { estado: 'error', error: 'Se perdió el contacto con el trabajo. Reinténtalo.' });
              registrarRef.current(foto, 'error', 'Sin contacto con el backend durante 1 min.');
              olvidarReintento(foto.id);
              return false;
            }
            continue;
          }
          nulos = 0;
          if (estado.estado === 'desconocido') {
            if (reenvios >= MAX_REENVIOS) {
              await marcarRef.current(foto, {
                estado: 'error',
                error: 'El backend se reinició y se perdió el trabajo. Reinténtalo.',
              });
              registrarRef.current(foto, 'error', 'Trabajo desconocido tras 3 reenvíos.');
              olvidarReintento(foto.id);
              return false;
            }
            reenvios += 1;
            const envio = await enviarFoto(foto.id, foto.original, prompt);
            if (!envio.ok || !envio.jobId) {
              const motivo = envio.motivo ?? 'No se pudo reenviar.';
              await marcarRef.current(foto, { estado: 'error', error: motivo });
              registrarRef.current(foto, 'error', motivo.slice(0, 200));
              olvidarReintento(foto.id);
              return false;
            }
            jobId = envio.jobId;
            await marcarRef.current(foto, { jobId: envio.jobId });
            registrarRef.current(foto, 'encolada', `Reenvío ${reenvios}/${MAX_REENVIOS} tras reinicio del backend.`);
            pausaSeg = 5;
            continue;
          }
          const intentosBackend = typeof estado.intentos === 'number' ? estado.intentos : 0;
          if (intentosBackend > ultimoIntentosVistos) {
            ultimoIntentosVistos = intentosBackend;
            if (estado.motivo && intentosBackend > 0) {
              registrarRef.current(foto, 'encolada', `Reintento ${intentosBackend}: ${estado.motivo}`.slice(0, 200));
            }
          }
          setReintentos((m) => ({
            ...m,
            [foto.id]: { intentos: intentosBackend, enSeg: estado.proximoReintentoEnSeg ?? null, motivo: estado.motivo ?? null },
          }));
          if (estado.estado === 'lista' && estado.imagen) {
            const actualizada = await marcarRef.current(foto, {
              estado: 'lista',
              mejorada: estado.imagen,
              error: null,
            });
            try {
              /* La mejora recién producida se guarda en el servidor de
               * inmediato: sin esto vive solo en IndexedDB y otro navegador
               * no la ve. El `orden` se relee en fresco: si el usuario
               * reordenó mientras volaba, el de la foto es viejo. */
              const ordenVigente = fotosRef.current.find((f) => f.id === foto.id)?.orden ?? foto.orden;
              await subirMejorada(foto.inmuebleId, estado.imagen, ordenVigente);
              registrarRef.current(actualizada, 'lista', 'Guardada en el servidor.');
            } catch {
              registrarRef.current(
                actualizada,
                'lista',
                'Solo en este navegador: no se pudo subir al servidor. Se conserva local.',
              );
            }
            esperaLocalRef.current.delete(foto.id);
            olvidarReintento(foto.id);
            return true;
          }
          if (estado.estado === 'error') {
            const motivo = estado.error ?? 'El backend no pudo mejorarla.';
            await marcarRef.current(foto, { estado: 'error', error: motivo });
            registrarRef.current(foto, 'error', motivo.slice(0, 200));
            esperaLocalRef.current.delete(foto.id);
            olvidarReintento(foto.id);
            return false;
          }
          if (estado.estado === 'cancelado') {
            // Cancelado por el usuario (en esta u otra pestaña): la foto
            // vuelve a pendiente sin mancha para poder reintentarse.
            await marcarRef.current(foto, { estado: 'pendiente', jobId: null, error: null });
            registrarRef.current(foto, 'cancelado', 'Cancelado por el usuario.');
            olvidarReintento(foto.id);
            return false;
          }
          // Encolado (a la espera o con reintento programado) o procesando:
          // se sigue esperando; si hay reintento futuro, se aparca el sondeo
          // (como mucho 60 s para reaccionar pronto a una cancelación).
          pausaSeg =
            estado.proximoReintentoEnSeg != null && estado.proximoReintentoEnSeg > 5
              ? Math.min(estado.proximoReintentoEnSeg, 60)
              : 5;
        }
      } finally {
        enVuelo.current = false;
        setOcupado(false);
      }
    },
    [olvidarReintento],
  );

  const procesarUna = useCallback(
    async (foto: FotoMejora, prompt: string): Promise<boolean> => {
      // Sin vuelo no hay seguidor: si otro seguimiento está en curso, la foto
      // queda pendiente (nunca huérfana en procesando) y se retoma luego.
      if (enVuelo.current) return false;
      setAviso(null);
      const actualizada = await marcarRef.current(foto, {
        estado: 'procesando',
        error: null,
        jobId: null,
        intentos: foto.intentos + 1,
      });
      registrarRef.current(actualizada, 'encolada', `Intento ${actualizada.intentos}.`);
      const envio = await enviarFoto(foto.id, foto.original, prompt);
      if (!envio.ok || !envio.jobId) {
        const motivo = envio.motivo ?? 'No se pudo encolar.';
        // A pendiente con backoff local (1-2-4-8-10 min): el automático la
        // retoma solo; en manual queda el motivo visible. Nada permanente
        // por una caída temporal del backend.
        const previo = esperaLocalRef.current.get(foto.id);
        const fallos = (previo?.fallos ?? 0) + 1;
        const esperaMin = Math.min(10, 2 ** (fallos - 1));
        esperaLocalRef.current.set(foto.id, { fallos, permitido: Date.now() + esperaMin * 60_000 });
        await marcarRef.current(foto, { estado: 'pendiente', error: `${motivo} Se reintentará solo en ~${esperaMin} min.` });
        registrarRef.current(foto, 'error', motivo.slice(0, 200));
        if (motivo.includes('Backend no disponible')) {
          setAviso('Backend no disponible. Arranca `npm run server:mejora` y reintenta.');
        }
        return false;
      }
      esperaLocalRef.current.delete(foto.id);
      await marcarRef.current(actualizada, { jobId: envio.jobId });
      return seguirTrabajo(actualizada, envio.jobId, prompt);
    },
    [seguirTrabajo],
  );

  // Reconciliación al tener fotos cargadas (llegan async desde IndexedDB:
  // al montar la lista aún está vacía). Retoma trabajos a medias de una
  // sesión anterior: con jobId → sigue el sondeo; sin jobId → a pendiente.
  useEffect(() => {
    if (reconciliado.current || fotos.length === 0) return;
    reconciliado.current = true;
    const base = fotosRef.current;
    const prompt = configRef.current.prompt;
    void (async () => {
      for (const foto of base) {
        if (foto.estado !== 'procesando') continue;
        if (foto.jobId) {
          await seguirTrabajo(foto, foto.jobId, prompt);
        } else {
          await marcarRef.current(foto, { estado: 'pendiente', error: null });
          registrarRef.current(foto, 'error', 'El seguimiento se interrumpió. Pulsa Reintentar.');
        }
      }
    })();
    // Se ejecuta una sola vez gracias a `reconciliado`; el seguimiento
    // posterior lo mueven los eventos (reintentar / modo automático).
  }, [fotos, seguirTrabajo]);

  // Automático: solo en modo automático. Respeta el backoff local y las
  // fotos pausadas por el usuario (Canceladas: solo Reintentar las mueve).
  // Dos despertadores: reacción inmediata a cambios de estado + latido cada
  // 10 s. El latido es el que retoma la cola cuando el backoff local vence:
  // sin él, una foto con espera programada quedaba pendiente para siempre
  // (nada volvía a ejecutar este efecto) y solo "reiniciar cola" la movía.
  useEffect(() => {
    if (config.modo !== 'automatico') return;
    const intentar = () => {
      if (enVuelo.current) return;
      const ahora = Date.now();
      const siguiente = fotosRef.current.find(
        (f) =>
          f.estado === 'pendiente' &&
          !pausadosRef.current.has(f.id) &&
          (esperaLocalRef.current.get(f.id)?.permitido ?? 0) <= ahora,
      );
      if (!siguiente) return;
      void procesarUna(siguiente, configRef.current.prompt);
    };
    intentar();
    /* [08AA-22] Latido vía plataforma (sin `window` directo aquí). */
    return repetirCada(10_000, intentar);
  }, [fotos, config.modo, config.prompt, procesarUna]);

  const reintentar = useCallback(
    async (foto: FotoMejora, prompt: string) => {
      // Reintentar despausa: el usuario la quiere mover.
      pausadosRef.current.delete(foto.id);
      canceladosRef.current.delete(foto.id);
      // Si ya hay un trabajo en curso para esta foto, retoma su seguimiento
      // en vez de encolar un duplicado en el backend.
      if (foto.estado === 'procesando' && foto.jobId) {
        await seguirTrabajo(foto, foto.jobId, prompt);
        return;
      }
      // Error final con trabajo conocido: reintento barato en el backend con
      // el original retenido, sin re-subir MBs. Si ya no existe (reinicio),
      // se cae al flujo normal de re-envío.
      if (foto.estado === 'error' && foto.jobId) {
        const barato = await reintentarTrabajo(foto.jobId);
        if (barato) {
          const base = await marcarRef.current(foto, { estado: 'procesando', error: null });
          registrarRef.current(base, 'encolada', 'Reintento sin re-subir (original retenido en el backend).');
          await seguirTrabajo(base, foto.jobId, prompt);
          return;
        }
      }
      // Manual: sin espera local; ahora mismo.
      esperaLocalRef.current.delete(foto.id);
      const base =
        foto.estado === 'procesando' ? foto : await marcarRef.current(foto, { estado: 'pendiente', error: null });
      await procesarUna(base, prompt);
    },
    [procesarUna, seguirTrabajo],
  );

  /* F21: cancelar el seguimiento de una foto. Si tiene trabajo en el
   * backend se cancela allí (sale de la cola o aborta el proceso); si no,
   * se pausa para que el automático no la toque. En ambos casos queda
   * pendiente limpia y Reintentar la mueve. */
  const cancelar = useCallback(
    async (foto: FotoMejora) => {
      canceladosRef.current.add(foto.id);
      pausadosRef.current.add(foto.id);
      if (foto.jobId) {
        const ok = await cancelarTrabajo(foto.jobId);
        registrarRef.current(
          foto,
          'cancelado',
          ok ? 'Cancelado en el backend: fuera de la cola.' : 'Sin contacto con el backend: queda pendiente.',
        );
      } else {
        registrarRef.current(foto, 'cancelado', 'Pausado: el automático no la tocará hasta Reintentar.');
      }
      await marcarRef.current(foto, { estado: 'pendiente', jobId: null, error: null });
      olvidarReintento(foto.id);
    },
    [olvidarReintento],
  );

  /* F21: empezar de nuevo sin tocar lo lista. Pone en pendiente (sin
   * jobId ni error) toda foto atascada en procesando o en error final;
   * devuelve cuántas se movieron. Lo lista no se toca. */
  const reiniciarTodo = useCallback(async () => {
    let movidas = 0;
    for (const foto of fotosRef.current) {
      if (foto.estado === 'lista') continue;
      if (foto.estado === 'pendiente' && !foto.jobId && !foto.error) continue;
      pausadosRef.current.delete(foto.id);
      canceladosRef.current.delete(foto.id);
      esperaLocalRef.current.delete(foto.id);
      const base = await marcarRef.current(foto, { estado: 'pendiente', error: null, jobId: null });
      registrarRef.current(base, 'cancelado', 'Reinicio de cola: vuelta a pendiente.');
      movidas += 1;
    }
    return movidas;
  }, []);

  return { ocupado, aviso, procesarUna, reintentar, cancelar, reiniciarTodo, reintentos };
}
