/* [147A-F5.9] Sub-hook extraído de useConfiguracion — sincronización servidor→local.
 * Cumple límite de 120 líneas (Regla 8). */

import { useState, useEffect } from 'react';
import { DEFAULTS } from './configuracion-types';
import type { EstadoConfiguracion } from './configuracion-types';

interface ConfigServerData {
  status: number;
  data: Record<string, string | number | boolean>;
}

export function useConfiguracionSync(datos: ConfigServerData | undefined) {
  const [config, setConfig] = useState<EstadoConfiguracion>(DEFAULTS);

  useEffect(() => {
    if (!datos || datos.status !== 200) return;
    const d = datos.data;
    setConfig({
      reserva_email_obligatorio: Boolean(d.reserva_email_obligatorio),
      reserva_telefono_obligatorio: Boolean(d.reserva_telefono_obligatorio),
      reserva_nombre_obligatorio: Boolean(d.reserva_nombre_obligatorio),
      reserva_apellidos_obligatorio: Boolean(d.reserva_apellidos_obligatorio),
      iva_por_defecto: Number(d.iva_por_defecto),
      nombre_restaurante: String(d.nombre_restaurante ?? ''),
      groq_api_key: config.groq_api_key || '',
      auto_venta_reserva: Boolean(d.auto_venta_reserva),
      hora_desayuno_inicio: String(d.hora_desayuno_inicio ?? '00:00:00'),
      hora_desayuno_fin: String(d.hora_desayuno_fin ?? '12:00:00'),
      hora_comida_inicio: String(d.hora_comida_inicio ?? '12:00:00'),
      hora_comida_fin: String(d.hora_comida_fin ?? '18:00:00'),
      hora_cena_inicio: String(d.hora_cena_inicio ?? '18:00:00'),
      hora_cena_fin: String(d.hora_cena_fin ?? '23:59:59'),
      url_haddock: String(d.url_haddock ?? ''),
      haddock_api_token: config.haddock_api_token || '',
      haddock_sync_enabled: Boolean(d.haddock_sync_enabled),
      bdp_base_url: String(d.bdp_base_url ?? ''),
      bdp_login: config.bdp_login || '',
      bdp_password: config.bdp_password || '',
      bdp_integrator_code: config.bdp_integrator_code || '',
      bdp_sync_enabled: Boolean(d.bdp_sync_enabled ?? false),
      bdp_pos_id: Number(d.bdp_pos_id ?? 1),
      bdp_employee_id: Number(d.bdp_employee_id ?? 1),
      bdp_items_profile_id: Number(d.bdp_items_profile_id ?? 1),
      bdp_default_article_code: String(d.bdp_default_article_code ?? ''),
      bdp_default_article_name: String(d.bdp_default_article_name ?? ''),
      bdp_tender_map: typeof d.bdp_tender_map === 'object' && d.bdp_tender_map !== null
        ? JSON.stringify(d.bdp_tender_map, null, 2)
        : String(d.bdp_tender_map ?? '{}'),
      bdp_order_type_map: typeof d.bdp_order_type_map === 'object' && d.bdp_order_type_map !== null
        ? JSON.stringify(d.bdp_order_type_map, null, 2)
        : String(d.bdp_order_type_map ?? '{}'),
      bdp_default_customer_code: String(d.bdp_default_customer_code ?? ''),
      bdp_poll_interval_secs: Number(d.bdp_poll_interval_secs ?? 60),
      bdp_poll_enabled: Boolean(d.bdp_poll_enabled ?? false),
      bdp_auto_sync_customers: Boolean(d.bdp_auto_sync_customers ?? false),
      /* [267A-4] Feature flags BDP — sincronizados desde servidor */
      ff_bdp_auto_arm: Boolean(d.ff_bdp_auto_arm ?? false),
      ff_bdp_partial_payments: Boolean(d.ff_bdp_partial_payments ?? false),
      ff_bdp_cancel_order: Boolean(d.ff_bdp_cancel_order ?? false),
      ff_bdp_purchase_notes_read: Boolean(d.ff_bdp_purchase_notes_read ?? false),
      ff_bdp_purchase_notes_draft: Boolean(d.ff_bdp_purchase_notes_draft ?? false),
      ff_bdp_purchase_notes_receive: Boolean(d.ff_bdp_purchase_notes_receive ?? false),
      bdp_sync_mode: String(d.bdp_sync_mode ?? 'read_only'),
      bdp_backup_retention_days: Number(d.bdp_backup_retention_days ?? 30),
      bdp_auto_backup_before_write: Boolean(d.bdp_auto_backup_before_write ?? false),
      google_review_url: String(d.google_review_url ?? ''),
      telefono_restaurante: String(d.telefono_restaurante ?? ''),
      url_reservas: String(d.url_reservas ?? ''),
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [datos]);

  return { config, setConfig };
}
