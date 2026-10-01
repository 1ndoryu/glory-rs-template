//! Gestión de fases de la orden (solo `PaymentMode::Phased` salvo entrega/aprobación).

use sqlx::PgPool;
use uuid::Uuid;

use super::OrderService;
use crate::errors::AppError;
use crate::models::{PaymentMode, PhaseStatus, UpdateOrderPhaseDefinitionRequest};
use crate::repositories::OrderRepository;

impl OrderService {
    pub async fn update_phase_definition(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
        req: UpdateOrderPhaseDefinitionRequest,
    ) -> Result<crate::models::OrderPhaseResponse, AppError> {
        use crate::models::UserRole;

        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.payment_mode != PaymentMode::Phased {
            return Err(AppError::BadRequest(
                "Solo las ordenes por fases admiten definicion manual de fases".into(),
            ));
        }

        if effective_role != UserRole::Admin && order.assigned_employee_id != Some(user_id) {
            return Err(AppError::Forbidden(
                "Solo el empleado asignado puede definir las fases".into(),
            ));
        }

        let phase = OrderRepository::find_phase_by_number(pool, order_id, phase_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

        /* [035A-30] También se permite editar fases in_progress (p.ej. ajustar precio/días) */
        if !matches!(
            phase.status,
            PhaseStatus::Locked | PhaseStatus::PendingPayment | PhaseStatus::InProgress
        ) {
            return Err(AppError::BadRequest(
                "Solo se pueden editar fases que no han sido entregadas o aprobadas".into(),
            ));
        }

        if req.title.is_none()
            && req.description.is_none()
            && req.price_cents.is_none()
            && req.estimated_days.is_none()
            && req.max_revisions.is_none()
        {
            return Err(AppError::BadRequest(
                "Debes enviar al menos un cambio para actualizar la fase".into(),
            ));
        }

        let title = req.title.as_deref().map(str::trim);
        if matches!(title, Some("")) {
            return Err(AppError::Validation(
                "El titulo de la fase no puede estar vacio".into(),
            ));
        }

        let description = req.description.as_deref().map(str::trim);
        if let Some(price_cents) = req.price_cents {
            if price_cents <= 0 {
                return Err(AppError::Validation(
                    "El precio de la fase debe ser mayor que cero".into(),
                ));
            }
        }

        if let Some(estimated_days) = req.estimated_days {
            if estimated_days <= 0 {
                return Err(AppError::Validation(
                    "Los dias estimados deben ser mayores que cero".into(),
                ));
            }
        }

        if let Some(max_revisions) = req.max_revisions {
            if max_revisions < 0 {
                return Err(AppError::Validation(
                    "Las revisiones maximas no pueden ser negativas".into(),
                ));
            }
        }

        let updated = OrderRepository::update_order_phase_definition(
            pool,
            phase.id,
            title,
            description,
            req.price_cents,
            req.estimated_days,
            req.max_revisions,
        )
        .await?;

        Ok(crate::models::OrderPhaseResponse::from(updated))
    }
    /* [035A-30] Agrega una nueva fase bloqueada al final de la orden (solo phased).
     * Validaciones: orden existe, payment_mode = Phased, usuario autorizado.
     * Gotcha: el empleado debe estar asignado; admin siempre puede. */
    pub async fn add_phase(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
    ) -> Result<crate::models::OrderPhaseResponse, AppError> {
        use crate::models::UserRole;

        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.payment_mode != PaymentMode::Phased {
            return Err(AppError::BadRequest(
                "Solo las órdenes por fases admiten gestión manual de fases".into(),
            ));
        }

        if effective_role != UserRole::Admin && order.assigned_employee_id != Some(user_id) {
            return Err(AppError::Forbidden(
                "Solo el empleado asignado o un administrador puede gestionar las fases".into(),
            ));
        }

        let max_num = OrderRepository::max_phase_number(pool, order_id).await?;
        let new_num = max_num + 1;
        let phase = OrderRepository::add_order_phase(pool, order_id, new_num).await?;
        Ok(crate::models::OrderPhaseResponse::from(phase))
    }

    /* [035A-30] Elimina una fase bloqueada de la orden (solo phased).
     * Solo se pueden eliminar fases en estado 'locked'.
     * Gotcha: el empleado debe estar asignado; admin siempre puede. */
    pub async fn delete_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
    ) -> Result<(), AppError> {
        use crate::models::UserRole;

        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.payment_mode != PaymentMode::Phased {
            return Err(AppError::BadRequest(
                "Solo las órdenes por fases admiten gestión manual de fases".into(),
            ));
        }

        if effective_role != UserRole::Admin && order.assigned_employee_id != Some(user_id) {
            return Err(AppError::Forbidden(
                "Solo el empleado asignado o un administrador puede gestionar las fases".into(),
            ));
        }

        let phase = OrderRepository::find_phase_by_number(pool, order_id, phase_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

        if phase.status != PhaseStatus::Locked {
            return Err(AppError::BadRequest(
                "Solo se pueden eliminar fases que aún no comenzaron (estado bloqueado)".into(),
            ));
        }

        let rows = OrderRepository::delete_order_phase(pool, order_id, phase_number).await?;
        if rows == 0 {
            return Err(AppError::NotFound(format!(
                "Fase {phase_number} no encontrada o no se puede eliminar"
            )));
        }

        Ok(())
    }
    /* [044A-38 Fase 2] Empleado entrega una fase.
     * Maquina de estados: InProgress | Paid | RevisionRequested -> Delivered.
     * Solo el empleado asignado puede entregar. */
    pub async fn deliver_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
        employee_id: Uuid,
    ) -> Result<crate::models::OrderPhase, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.assigned_employee_id != Some(employee_id) {
            return Err(AppError::Forbidden(
                "Solo el empleado asignado puede entregar".into(),
            ));
        }

        let phase = OrderRepository::find_phase_by_number(pool, order_id, phase_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

        match phase.status {
            PhaseStatus::InProgress | PhaseStatus::Paid | PhaseStatus::RevisionRequested => {}
            _ => {
                return Err(AppError::BadRequest(format!(
                    "No se puede entregar una fase en estado {:?}",
                    phase.status
                )));
            }
        }

        let delivered = OrderRepository::deliver_phase(pool, phase.id).await?;
        Ok(delivered)
    }

    /* [044A-38 Fase 2] Cliente aprueba una fase.
     * Maquina de estados: Delivered -> Approved.
     * Si es la ultima fase, marca la orden como Completed.
     * Si hay siguiente fase, la desbloquea (Locked -> PendingPayment o Paid segun modo). */
    pub async fn approve_phase(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
        client_id: Uuid,
    ) -> Result<crate::models::OrderPhase, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.client_id != client_id {
            return Err(AppError::Forbidden(
                "Solo el cliente puede aprobar fases".into(),
            ));
        }

        let phase = OrderRepository::find_phase_by_number(pool, order_id, phase_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

        if phase.status != PhaseStatus::Delivered {
            return Err(AppError::BadRequest(format!(
                "Solo se pueden aprobar fases entregadas (estado actual: {:?})",
                phase.status
            )));
        }

        let approved = OrderRepository::approve_phase(pool, phase.id).await?;

        /* Desbloquear siguiente fase o completar la orden */
        let all_phases = OrderRepository::list_order_phases(pool, order_id).await?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total = all_phases.len() as i32;
        let all_approved = all_phases
            .iter()
            .all(|p| p.status == PhaseStatus::Approved || p.id == phase.id);

        if all_approved {
            OrderRepository::complete_order(pool, order_id).await?;
        } else {
            /* Desbloquear siguiente fase */
            let next_number = phase_number + 1;
            if let Some(next_phase) = all_phases.iter().find(|p| p.phase_number == next_number) {
                if next_phase.status == PhaseStatus::Locked {
                    let unlock_status = Self::initial_phase_status(order.payment_mode);
                    OrderRepository::update_phase_status(pool, next_phase.id, unlock_status)
                        .await?;
                }
            }
            if next_number <= total {
                OrderRepository::update_current_phase(pool, order_id, next_number).await?;
            }
        }

        Ok(approved)
    }

    /* [044A-38 Fase 2] Cliente solicita revision.
     * Maquina de estados: Delivered -> RevisionRequested.
     * Valida que no se excedan las revisiones maximas. */
    pub async fn request_revision(
        pool: &PgPool,
        order_id: Uuid,
        phase_number: i32,
        client_id: Uuid,
    ) -> Result<crate::models::OrderPhase, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.client_id != client_id {
            return Err(AppError::Forbidden(
                "Solo el cliente puede solicitar revision".into(),
            ));
        }

        let phase = OrderRepository::find_phase_by_number(pool, order_id, phase_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Fase {phase_number} no encontrada")))?;

        if phase.status != PhaseStatus::Delivered {
            return Err(AppError::BadRequest(format!(
                "Solo se puede pedir revision en fases entregadas (estado actual: {:?})",
                phase.status
            )));
        }

        if phase.revisions_used >= phase.max_revisions {
            return Err(AppError::BadRequest(format!(
                "Se alcanzo el limite de revisiones ({}/{})",
                phase.revisions_used, phase.max_revisions
            )));
        }

        let revised = OrderRepository::request_revision(pool, phase.id).await?;
        Ok(revised)
    }
}
