//! Ciclo de vida de la orden: crear, leer, listar, asignar, cancelar, briefing.

use sqlx::PgPool;
use uuid::Uuid;

use super::OrderService;
use crate::errors::AppError;
use crate::models::{
    CreateOrderRequest, Order, OrderPhaseResponse, OrderResponse, OrderStatus, PaymentMode,
    PhaseStatus,
};
use crate::repositories::{
    CreateOrderParams, CreatePhaseParams, OrderRepository, ServiceRepository,
};
use crate::services::order_slugs::{find_plan_for_order, find_service_for_order};

impl OrderService {
    /// Crea una orden: resuelve servicio/plan, calcula descuento, genera fases
    pub async fn create_order(
        pool: &PgPool,
        client_id: Uuid,
        req: CreateOrderRequest,
    ) -> Result<OrderResponse, AppError> {
        let CreateOrderRequest {
            service_slug,
            plan_slug,
            payment_mode,
            project_description,
            client_notes,
        } = req;

        let svc = find_service_for_order(pool, &service_slug).await?;
        let plan = find_plan_for_order(pool, svc.id, &plan_slug).await?;

        let base_price = plan.price_cents;
        let discount = Self::discount_for_mode(payment_mode);

        /* [SEO-D] Descuento 50% primer pedido: si el usuario no tiene órdenes previas,
         * aplicar 50% de descuento. No acumula con payment_mode — se usa el mayor. */
        let first_order_discount = Self::first_order_discount_percent(pool, client_id).await?;
        let effective_discount = discount.max(first_order_discount);
        let final_price = base_price - (base_price * effective_discount / 100);

        let plan_phases = ServiceRepository::list_plan_phases(pool, plan.id).await?;

        /* [035A-10] En pagos por fases, el CMS define la estructura de trabajo.
         * No se deben crear órdenes con fases vacías ni sobrescribir plantillas del plan
         * con placeholders genéricos, porque eso rompe el contrato con el catálogo. */
        if payment_mode == PaymentMode::Phased && plan_phases.is_empty() {
            return Err(AppError::BadRequest(
                "Este plan no tiene fases configuradas en el CMS para pago por fases".into(),
            ));
        }

        let order = OrderRepository::create_order(
            pool,
            CreateOrderParams {
                client_id,
                service_id: svc.id,
                plan_id: plan.id,
                payment_mode,
                base_price_cents: base_price,
                discount_percent: effective_discount,
                final_price_cents: final_price,
                project_description: project_description.as_deref(),
                client_notes: client_notes.as_deref(),
            },
        )
        .await?;

        /* Generar fases de la orden a partir de las plantillas del plan */
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total_phases = plan_phases.len() as i32;
        for tmpl in &plan_phases {
            let phase_price = final_price * tmpl.percentage_of_total / 100;
            let status = if tmpl.phase_number == 1 {
                Self::initial_phase_status(payment_mode)
            } else {
                PhaseStatus::Locked
            };

            OrderRepository::create_order_phase(
                pool,
                CreatePhaseParams {
                    order_id: order.id,
                    phase_number: tmpl.phase_number,
                    title: &tmpl.title,
                    description: tmpl.description.as_deref(),
                    price_cents: phase_price,
                    status,
                    max_revisions: tmpl.max_revisions,
                    estimated_days: tmpl.estimated_days,
                },
            )
            .await?;
        }

        Ok(OrderResponse {
            id: order.id,
            order_number: order.order_number,
            client_id: order.client_id,
            client_name: None,
            service_title: svc.title,
            service_slug: svc.slug,
            plan_name: plan.name,
            payment_mode: order.payment_mode,
            base_price_cents: order.base_price_cents,
            discount_percent: order.discount_percent,
            final_price_cents: order.final_price_cents,
            currency: order.currency,
            status: order.status,
            assigned_employee_id: order.assigned_employee_id,
            assigned_employee_name: None,
            current_phase: order.current_phase,
            total_phases,
            project_description: order.project_description,
            client_notes: order.client_notes,
            started_at: order.started_at,
            created_at: order.created_at,
            ai_intermediary_enabled: order.ai_intermediary_enabled.unwrap_or(false),
            ai_summary: order.ai_summary,
        })
    }

    /// Obtiene el detalle de una orden con sus fases.
    /// Retorna (`client_id`, response, phases) para que el handler verifique acceso.
    pub async fn get_order(
        pool: &PgPool,
        order_id: Uuid,
    ) -> Result<(Uuid, OrderResponse, Vec<OrderPhaseResponse>), AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        let phases = OrderRepository::list_order_phases(pool, order_id).await?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total_phases = phases.len() as i32;

        let (svc_title, svc_slug, plan_name) =
            OrderRepository::get_order_display_info(pool, order.service_id, order.plan_id).await?;

        let employee_name =
            OrderRepository::get_employee_display_name(pool, order.assigned_employee_id).await?;
        let client_name = OrderRepository::get_client_display_name(pool, order.client_id).await?;

        let response = OrderResponse {
            id: order.id,
            order_number: order.order_number,
            client_id: order.client_id,
            client_name,
            service_title: svc_title,
            service_slug: svc_slug,
            plan_name,
            payment_mode: order.payment_mode,
            base_price_cents: order.base_price_cents,
            discount_percent: order.discount_percent,
            final_price_cents: order.final_price_cents,
            currency: order.currency,
            status: order.status,
            assigned_employee_id: order.assigned_employee_id,
            assigned_employee_name: employee_name,
            current_phase: order.current_phase,
            total_phases,
            project_description: order.project_description,
            client_notes: order.client_notes,
            started_at: order.started_at,
            created_at: order.created_at,
            ai_intermediary_enabled: order.ai_intermediary_enabled.unwrap_or(false),
            ai_summary: order.ai_summary.clone(),
        };

        let phase_responses: Vec<OrderPhaseResponse> =
            phases.into_iter().map(OrderPhaseResponse::from).collect();

        Ok((order.client_id, response, phase_responses))
    }

    /// Lista ordenes segun rol: cliente ve las suyas, employee las asignadas, admin todas
    pub async fn list_orders_for_user(
        pool: &PgPool,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
    ) -> Result<Vec<OrderResponse>, AppError> {
        use crate::models::UserRole;

        let orders = match effective_role {
            UserRole::Client => OrderRepository::list_orders_for_client(pool, user_id).await?,
            UserRole::Employee => OrderRepository::list_orders_for_employee(pool, user_id).await?,
            UserRole::Admin => OrderRepository::list_all_orders(pool).await?,
        };

        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            let (svc_title, svc_slug, plan_name) =
                OrderRepository::get_order_display_info(pool, order.service_id, order.plan_id)
                    .await?;
            let phases = OrderRepository::list_order_phases(pool, order.id).await?;
            let employee_name =
                OrderRepository::get_employee_display_name(pool, order.assigned_employee_id)
                    .await?;
            let client_name =
                OrderRepository::get_client_display_name(pool, order.client_id).await?;

            result.push(OrderResponse {
                id: order.id,
                order_number: order.order_number,
                client_id: order.client_id,
                client_name,
                service_title: svc_title,
                service_slug: svc_slug,
                plan_name,
                payment_mode: order.payment_mode,
                base_price_cents: order.base_price_cents,
                discount_percent: order.discount_percent,
                final_price_cents: order.final_price_cents,
                currency: order.currency,
                status: order.status,
                assigned_employee_id: order.assigned_employee_id,
                assigned_employee_name: employee_name,
                current_phase: order.current_phase,
                total_phases: {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    {
                        phases.len() as i32
                    }
                },
                project_description: order.project_description,
                client_notes: order.client_notes,
                started_at: order.started_at,
                created_at: order.created_at,
                ai_intermediary_enabled: order.ai_intermediary_enabled.unwrap_or(false),
                ai_summary: order.ai_summary,
            });
        }

        Ok(result)
    }

    /// Asigna un empleado a una orden (admin o auto-asignacion)
    pub async fn assign_order(
        pool: &PgPool,
        order_id: Uuid,
        employee_id: Uuid,
    ) -> Result<Order, AppError> {
        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        if order.status != OrderStatus::AwaitingAssignment
            && order.status != OrderStatus::PaymentHeld
        {
            return Err(AppError::BadRequest(
                "La orden no esta en estado de asignacion".into(),
            ));
        }

        let assigned = OrderRepository::assign_order(pool, order_id, employee_id).await?;
        Ok(assigned)
    }

    /* [104A-28] Cancela una orden. Empleados pueden cancelar con razon obligatoria.
     * Maquina de estados: PendingPayment | PaymentHeld | AwaitingAssignment | InProgress -> Cancelled */
    pub async fn cancel_order(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
        reason: Option<&str>,
    ) -> Result<Order, AppError> {
        use crate::models::UserRole;

        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        /* Verificar permisos: dueno, empleado asignado (con razon), o admin */
        match effective_role {
            UserRole::Admin => {}
            UserRole::Client => {
                if order.client_id != user_id {
                    return Err(AppError::Forbidden(
                        "No tienes permiso para cancelar esta orden".into(),
                    ));
                }
            }
            UserRole::Employee => {
                if order.assigned_employee_id != Some(user_id) {
                    return Err(AppError::Forbidden("No estas asignado a esta orden".into()));
                }
                if reason.is_none_or(|r| r.trim().is_empty()) {
                    return Err(AppError::Validation(
                        "Los empleados deben proporcionar una razon para cancelar".into(),
                    ));
                }
            }
        }

        /* [104A-29] Solo se puede cancelar en estados iniciales + in_progress para empleados.
         * pending_payment ya no se usa a nivel de orden; solo en fases. */
        let can_cancel = matches!(
            order.status,
            OrderStatus::PaymentHeld | OrderStatus::AwaitingAssignment
        ) || (effective_role == UserRole::Employee
            && order.status == OrderStatus::InProgress);

        if !can_cancel {
            return Err(AppError::BadRequest(format!(
                "No se puede cancelar una orden en estado {:?}",
                order.status
            )));
        }

        /* Guardar razon si se proporciono */
        if let Some(r) = reason {
            sqlx::query!(
                "UPDATE orders SET cancel_reason = $1 WHERE id = $2",
                r,
                order_id,
            )
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("Error guardando razon: {e}")))?;
        }

        let cancelled = OrderRepository::cancel_order(pool, order_id).await?;
        Ok(cancelled)
    }

    pub async fn update_project_description(
        pool: &PgPool,
        order_id: Uuid,
        user_id: Uuid,
        effective_role: crate::models::UserRole,
        project_description: String,
    ) -> Result<OrderResponse, AppError> {
        use crate::models::UserRole;

        let order = OrderRepository::find_order_by_id(pool, order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Orden no encontrada".into()))?;

        /* [035A-15] project_description es editable por admin o por el empleado asignado.
         * El cliente conserva lectura, pero no puede reescribir el briefing operativo. */
        let can_edit = match effective_role {
            UserRole::Admin => true,
            UserRole::Employee => order.assigned_employee_id == Some(user_id),
            UserRole::Client => false,
        };

        if !can_edit {
            return Err(AppError::Forbidden(
                "Solo el empleado asignado o un admin puede actualizar la descripción del proyecto"
                    .into(),
            ));
        }

        if matches!(
            order.status,
            OrderStatus::Completed | OrderStatus::Cancelled
        ) {
            return Err(AppError::BadRequest(
                "La descripcion no se puede editar en una orden cerrada".into(),
            ));
        }

        let updated =
            OrderRepository::update_project_description(pool, order_id, &project_description)
                .await?;
        let phases = OrderRepository::list_order_phases(pool, order_id).await?;
        let (svc_title, svc_slug, plan_name) =
            OrderRepository::get_order_display_info(pool, updated.service_id, updated.plan_id)
                .await?;
        let employee_name =
            OrderRepository::get_employee_display_name(pool, updated.assigned_employee_id).await?;
        let client_name = OrderRepository::get_client_display_name(pool, updated.client_id).await?;

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let total_phases = phases.len() as i32;

        Ok(OrderResponse {
            id: updated.id,
            order_number: updated.order_number,
            client_id: updated.client_id,
            client_name,
            service_title: svc_title,
            service_slug: svc_slug,
            plan_name,
            payment_mode: updated.payment_mode,
            base_price_cents: updated.base_price_cents,
            discount_percent: updated.discount_percent,
            final_price_cents: updated.final_price_cents,
            currency: updated.currency,
            status: updated.status,
            assigned_employee_id: updated.assigned_employee_id,
            assigned_employee_name: employee_name,
            current_phase: updated.current_phase,
            total_phases,
            project_description: updated.project_description,
            client_notes: updated.client_notes,
            started_at: updated.started_at,
            created_at: updated.created_at,
            ai_intermediary_enabled: updated.ai_intermediary_enabled.unwrap_or(false),
            ai_summary: updated.ai_summary,
        })
    }
}
