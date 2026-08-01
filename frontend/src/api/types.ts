/* wandori.us — API Types
 * Tipos TypeScript para las respuestas de la API.
 * La superficie se migra gradualmente a los contratos generados por Orval. */

/* === Auth === */
export interface LoginRequest {
  email: string;
  password: string;
}

/* === Articles === */
export interface Article {
  id: string;
  title: string;
  slug: string;
  content: Record<string, unknown>; /* TipTap JSON */
  excerpt: string;
  cover_image: string | null;
  status: 'draft' | 'published';
  is_pinned: boolean;
  published_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateArticleRequest {
  title: string;
  content: Record<string, unknown>;
  excerpt?: string;
  cover_image?: string;
  status?: 'draft' | 'published';
  is_pinned?: boolean;
}

export interface UpdateArticleRequest {
  title?: string;
  content?: Record<string, unknown>;
  excerpt?: string;
  cover_image?: string;
  status?: 'draft' | 'published';
  is_pinned?: boolean;
}

export interface PaginatedArticles {
  items: Article[];
  total: number;
  page: number;
  per_page: number;
}

/* === Media === */
export type AssetState = 'processing' | 'clean' | 'rejected';

export interface MediaPublic {
  url: string;
  file_name: string;
  file_type: 'image' | 'audio' | 'video';
  file_size: number;
  alt_text: string;
  created_at: string;
}

export interface MediaAdmin extends MediaPublic {
  id: string;
  article_id: string | null;
  /** [297A-10] Estado de procesamiento del asset. */
  asset_state: AssetState;
  /** Preview protegido para assets privados o no publicados. */
  admin_url: string;
}

/* La subida conserva la misma metadata base que la biblioteca admin. */
export type MediaUpload = MediaAdmin;

/* === Products === */
export interface Product {
  id: string;
  article_id: string | null;
  name: string;
  description: string;
  price_cents: number;
  currency: string;
  is_active: boolean;
  created_at: string;
}

export interface CreateProductRequest {
  article_id?: string;
  name: string;
  description?: string;
  price_cents: number;
  currency?: string;
  is_active?: boolean;
}

export interface UpdateProductRequest {
  name?: string;
  description?: string;
  price_cents?: number;
  currency?: string;
  is_active?: boolean;
}

/* === Orders === */
export interface Order {
  id: string;
  product_id: string;
  customer_email: string;
  status: 'pending' | 'paid' | 'delivered' | 'failed';
  paid_at: string | null;
  delivered_at: string | null;
  created_at: string;
}

/* === Projects === */
export interface Project {
  id: string;
  title: string;
  description: string;
  url: string | null;
  sort_order: number;
  is_visible: boolean;
  created_at: string;
}

export interface CreateProjectRequest {
  title: string;
  description?: string;
  url?: string;
  sort_order?: number;
  is_visible?: boolean;
}

export interface UpdateProjectRequest {
  title?: string;
  description?: string;
  url?: string | null;
  sort_order?: number;
  is_visible?: boolean;
}

/* === Settings === */
export interface SiteSettings {
  [key: string]: string;
}

/* === Analytics === */
export interface AnalyticsEvent {
  event_id?: string;
  event_type: string;
  target_type?: string;
  target_id?: string;
  metadata?: Record<string, unknown>;
}

export interface AnalyticsStats {
  total_page_views: number;
  total_clicks: number;
  total_downloads: number;
  total_purchases: number;
  top_articles: Array<{ id: string; title: string; views: number }>;
  recent_events: Array<{
    event_type: string;
    target_type: string;
    created_at: string;
  }>;
}
