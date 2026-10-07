// Config del chat (169A-5; [07AA-1 F4] todo-controlable por bloques):
// prompt extra, teléfonos, aviso WhatsApp, kill-switch global, tools,
// números A/B, ventana, autorizados, tope, corte y tono. Guarda lo cambiado.

import { useConfigChat } from '../../hooks/chat/use-config-chat';
import type { ClaveConfig } from '../../data/chat/cliente-admin';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';

interface Campo {
  clave: ClaveConfig;
  etiqueta: string;
  ayuda: string;
  multilinea?: boolean;
}

interface Bloque {
  titulo: string;
  campos: Campo[];
}

/* Mismo orden que `CLAVES_CONFIG` del backend; sin estilos nuevos: reusa
 * `Input`/`Textarea`/`Button` del sistema. */
const BLOQUES: Bloque[] = [
  {
    titulo: 'Números WhatsApp',
    campos: [
      { clave: 'wa_numero_a', etiqueta: 'Número A (canónico, 0412)', ayuda: 'El asistente general: mismo de la web. Dígitos con prefijo.' },
      { clave: 'wa_numero_b', etiqueta: 'Número B (temporal, a jubilar)', ayuda: 'Solo mudo/inicial hasta F5. Dígitos con prefijo.' },
      { clave: 'contacto_telefono', etiqueta: 'Teléfono de contacto público', ayuda: 'Lo da la IA y sale en la ficha del chat.' },
    ],
  },
  {
    titulo: 'Autorizados',
    campos: [
      { clave: 'whatsapp_admin', etiqueta: 'WhatsApp que recibe los avisos', ayuda: 'Dígitos con prefijo, p. ej. +34600111222.' },
      { clave: 'whatsapp_autorizados', etiqueta: 'Autorizados (staff por WhatsApp)', ayuda: 'Comas, con o sin +/espacios/guiones. Vacío = nadie.' },
    ],
  },
  {
    titulo: 'IA global',
    campos: [
      {
        clave: 'ai_enabled_global',
        etiqueta: 'IA global (on|off)',
        ayuda: 'Kill-switch: en off ningún hilo responde con IA.',
      },
      {
        clave: 'tools_deshabilitadas',
        etiqueta: 'Tools deshabilitadas',
        ayuda: 'Nombres separados por comas, p. ej. escalar_a_humano.',
      },
      { clave: 'prompt_extra', etiqueta: 'Instrucciones extra para la IA', ayuda: 'Se añade al prompt inmobiliario en cada turno (hoy sin lector: deuda).', multilinea: true },
    ],
  },
  {
    titulo: 'Tono WhatsApp',
    campos: [
      { clave: 'whatsapp_acuse_texto', etiqueta: 'Acuse de turno lento', ayuda: 'Vacío = fábrica. Máx 500 caracteres.' },
      { clave: 'whatsapp_fallback_texto', etiqueta: 'Fallback de turno fallido', ayuda: 'Vacío = fábrica. Máx 500 caracteres.' },
      { clave: 'whatsapp_aviso_asesor_texto', etiqueta: 'Aviso de escalada a asesor', ayuda: 'Vacío = fábrica. Máx 500 caracteres.' },
    ],
  },
  {
    titulo: 'Límites y corte',
    campos: [
      { clave: 'ventana_retraso_min', etiqueta: 'Ventana de retraso (min)', ayuda: 'Antigüedad máx del mensaje: 1–1440, fábrica 10.' },
      { clave: 'ia_tope_tokens_dia', etiqueta: 'Tope diario LLM (tokens)', ayuda: 'Solo alerta, nunca apaga. Fábrica 2000000.' },
      { clave: 'corte_whatsapp', etiqueta: 'Corte idempotencia (vacío|total|wa_b|apagado)', ayuda: 'Base de F5: hoy solo cubre reintentos, no bloquea envíos.' },
    ],
  },
];

export function ConfigChat() {
  const c = useConfigChat();
  /* Local para que TS estreche el nulo dentro de los callbacks. */
  const valores = c.valores;

  if (c.cargando || !valores) {
    return (
      <p className="rounded-lg border border-dashed px-6 py-16 text-center text-sm text-muted-foreground">
        Cargando configuración…
      </p>
    );
  }

  return (
    <div className="max-w-2xl space-y-4">
      {c.error && (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {c.error}
        </p>
      )}
      {c.aviso && (
        <p className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">{c.aviso}</p>
      )}
      {BLOQUES.map((bloque) => (
        <section key={bloque.titulo} className="space-y-4">
          <h3 className="text-sm font-semibold">{bloque.titulo}</h3>
          {bloque.campos.map((campo) => (
            <div key={campo.clave}>
              <label htmlFor={`config-${campo.clave}`} className="mb-1 block text-sm font-medium">
                {campo.etiqueta}
              </label>
              {campo.multilinea ? (
                <Textarea
                  id={`config-${campo.clave}`}
                  value={valores[campo.clave] ?? ''}
                  onChange={(e) => c.poner(campo.clave, e.target.value)}
                  rows={4}
                />
              ) : (
                <Input
                  id={`config-${campo.clave}`}
                  value={valores[campo.clave] ?? ''}
                  onChange={(e) => c.poner(campo.clave, e.target.value)}
                />
              )}
              <p className="mt-1 text-xs text-muted-foreground">{campo.ayuda}</p>
            </div>
          ))}
        </section>
      ))}
      <div className="flex gap-2">
        <Button onClick={() => void c.guardar()} disabled={c.guardando}>
          {c.guardando ? 'Guardando…' : 'Guardar cambios'}
        </Button>
        <Button variant="outline" onClick={() => void c.recargar()}>
          Recargar
        </Button>
      </div>
    </div>
  );
}
