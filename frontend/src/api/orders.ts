/* [044A-38 Fase 2] API client para órdenes del marketplace.
 * Conecta con todos los endpoints CRUD de /api/orders del backend Rust.
 * Tipos alineados con OrderResponse y OrderPhaseResponse del backend. */
import instance from './axios-instance';

/*    TIPOS — alineados con backend Rust (serde snake_case) */

export type OrderStatus =
    | 'pending_payment'
    | 'payment_held'
    | 'awaiting_assignment'
    | 'in_progress'
    | 'under_review'
    | 'completed'
    | 'cancelled'
    | 'disputed';

export type PaymentMode = 'full' | 'half_half' | 'phased';

export type PhaseStatus =
    | 'locked'
    | 'pending_payment'
    | 'paid'
    | 'in_progress'
    | 'delivered'
    | 'revision_requested'
    | 'approved'
    | 'skipped';

export interface OrderResponse {
    id: string;
    order_number: number;
    client_id: string;
    client_name: string | null;
    service_title: string;
    service_slug: string;
    plan_name: string;
    payment_mode: PaymentMode;
    base_price_cents: number;
    discount_percent: number;
    final_price_cents: number;
    currency: string;
    status: OrderStatus;
    assigned_employee_id: string | null;
    assigned_employee_name: string | null;
    current_phase: number;
    total_phases: number;
    project_description: string | null;
    client_notes: string | null;
    started_at: string | null;
    created_at: string;
    /* [T-10] IA intermediaria */
    ai_intermediary_enabled: boolean;
    ai_summary: string | null;
}

export interface OrderPhaseResponse {
    phase_number: number;
    title: string;
    description: string | null;
    price_cents: number;
    status: PhaseStatus;
    max_revisions: number;
    revisions_used: number;
    estimated_days: number;
    deadline: string | null;
}

export interface OrderDetailResponse {
    order: OrderResponse;
    phases: OrderPhaseResponse[];
}

export interface CreateOrderRequest {
    service_slug: string;
    plan_slug: string;
    payment_mode: PaymentMode;
    project_description?: string;
    client_notes?: string;
}

export interface UpdateOrderProjectDescriptionRequest {
    project_description: string;
}

export interface UpdateOrderPhaseDefinitionRequest {
    title?: string;
    description?: string;
    price_cents?: number;
    estimated_days?: number;
    max_revisions?: number;
}

/* [095A-19] La API de ordenes acepta aliases legacy en backend.
 * El frontend solo limpia espacios/case para no convertir slugs vivos del CMS
 * (`diseno-de-sitios-web`) a slugs antiguos (`diseno-web`) que causan 404. */
function normalizeOrderSlug(slug: string): string {
    return slug.trim().toLowerCase();
}

/*    API CALLS */

export async function apiListOrders(): Promise<OrderResponse[]> {
    const {data} = await instance.get<OrderResponse[]>('/api/orders');
    return data;
}

export async function apiGetOrder(orderId: string): Promise<OrderDetailResponse> {
    const {data} = await instance.get<OrderDetailResponse>(`/api/orders/${orderId}`);
    return data;
}

export async function apiCreateOrder(req: CreateOrderRequest): Promise<OrderResponse> {
    const normalizedRequest: CreateOrderRequest = {
        ...req,
        service_slug: normalizeOrderSlug(req.service_slug),
        plan_slug: normalizeOrderSlug(req.plan_slug),
    };
    const {data} = await instance.post<OrderResponse>('/api/orders', normalizedRequest);
    return data;
}

export async function apiUpdateOrderProjectDescription(
    orderId: string,
    req: UpdateOrderProjectDescriptionRequest,
): Promise<OrderResponse> {
    const {data} = await instance.patch<OrderResponse>(
        `/api/orders/${orderId}/project-description`,
        req,
    );
    return data;
}

export async function apiUpdateOrderPhaseDefinition(
    orderId: string,
    phaseNumber: number,
    req: UpdateOrderPhaseDefinitionRequest,
): Promise<OrderPhaseResponse> {
    const {data} = await instance.patch<OrderPhaseResponse>(
        `/api/orders/${orderId}/phases/${phaseNumber}`,
        req,
    );
    return data;
}

export async function apiCancelOrder(orderId: string, reason?: string): Promise<{status: OrderStatus}> {
    const {data} = await instance.post<{status: OrderStatus}>(
        `/api/orders/${orderId}/cancel`,
        reason ? {reason} : undefined,
    );
    return data;
}

export async function apiApprovePhase(
    orderId: string,
    phaseNumber: number,
): Promise<OrderPhaseResponse> {
    const {data} = await instance.put<OrderPhaseResponse>(
        `/api/orders/${orderId}/phases/${phaseNumber}/approve`,
    );
    return data;
}

export async function apiRequestRevision(
    orderId: string,
    phaseNumber: number,
): Promise<OrderPhaseResponse> {
    const {data} = await instance.put<OrderPhaseResponse>(
        `/api/orders/${orderId}/phases/${phaseNumber}/revision`,
    );
    return data;
}

/* [044A-38 Fase 6] apiDeliverPhase eliminada: la entrega ahora es multipart
 * y vive en api/deliverables.ts (apiDeliverPhase con FormData). */

/* [T-10] Toggle IA intermediaria para una orden */
export async function apiToggleAiIntermediary(
    orderId: string,
    enabled: boolean,
): Promise<OrderResponse> {
    const {data} = await instance.put<OrderResponse>(
        `/api/orders/${orderId}/ai-intermediary`,
        {enabled},
    );
    return data;
}

/* [035A-30] Agrega una nueva fase bloqueada al final de la orden phased. */
export async function apiAddOrderPhase(orderId: string): Promise<OrderPhaseResponse> {
    const {data} = await instance.post<OrderPhaseResponse>(`/api/orders/${orderId}/phases`);
    return data;
}

/* [035A-30] Elimina una fase bloqueada de la orden phased. */
export async function apiDeleteOrderPhase(orderId: string, phaseNumber: number): Promise<void> {
    await instance.delete(`/api/orders/${orderId}/phases/${phaseNumber}`);
}

/*    HELPERS — labels y colores para UI */

export const ORDER_STATUS_LABELS: Record<OrderStatus, string> = {
    /* [104A-29] pending_payment ya no se usa en órdenes nuevas — legacy label */
    pending_payment: 'Pendiente de pago',
    payment_held: 'Pendiente de pago',
    awaiting_assignment: 'Sin asignar',
    in_progress: 'En progreso',
    under_review: 'En revisión',
    completed: 'Completada',
    cancelled: 'Cancelada',
    disputed: 'Disputada',
};

export const ORDER_STATUS_COLOR: Record<OrderStatus, string> = {
    pending_payment: '#0077b6',
    payment_held: '#0077b6',
    awaiting_assignment: '#7b2cbf',
    in_progress: '#0077b6',
    under_review: '#c59000',
    completed: '#2d6a4f',
    cancelled: '#6c757d',
    disputed: '#d62828',
};

export const PHASE_STATUS_LABELS: Record<PhaseStatus, string> = {
    locked: 'Bloqueada',
    pending_payment: 'Pendiente de pago',
    paid: 'Pagada',
    in_progress: 'En progreso',
    delivered: 'Entregada',
    revision_requested: 'Revisión solicitada',
    approved: 'Aprobada',
    skipped: 'Omitida',
};

export const PAYMENT_MODE_LABELS: Record<PaymentMode, string> = {
    full: 'Pago único',
    half_half: '50/50',
    phased: 'Por fases',
};

/* Formato precio en centavos → string legible */
export function formatPrice(cents: number, currency = 'USD'): string {
    return new Intl.NumberFormat('es', {
        style: 'currency',
        currency,
        minimumFractionDigits: 0,
        maximumFractionDigits: 2,
    }).format(cents / 100);
}

/* [154A-15d] Timeline de actividad de una orden */
export interface ActivityEntry {
    id: string;
    user_id: string | null;
    action: string;
    details: Record<string, unknown> | null;
    created_at: string;
}

export async function apiGetOrderActivity(orderId: string): Promise<ActivityEntry[]> {
    const {data} = await instance.get<ActivityEntry[]>(`/api/orders/${orderId}/activity`);
    return data;
}

/* [277A-10] Verificar si el usuario califica para descuento de primer pedido (50% OFF) */
export interface FirstOrderDiscount {
    qualifies: boolean;
    discount_percent: number;
}

export async function apiCheckFirstOrderDiscount(): Promise<FirstOrderDiscount> {
    const {data} = await instance.get<FirstOrderDiscount>('/api/orders/first-order-discount');
    return data;
}
