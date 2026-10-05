/* [054A-2] API client de hosting: suscripciones y eventos.
 * Endpoints bajo /api/hosting/. Requiere JWT.
 * [01AA-4-f3s] Barril del split de api/hosting.ts (1049L, 872 efectivas →
 * limite-lineas-nivel-2): el código vive en submódulos por subdominio y este
 * archivo solo re-exporta para no romper los ~30 imports existentes.
 * En código nuevo, importar desde el submódulo concreto. */

export * from './hostingCore';
export * from './hostingOps';
export * from './hostingDeployments';
export * from './hostingVps';
export * from './hostingDomains';
export * from './hostingBackups';
export * from './hostingEmail';
