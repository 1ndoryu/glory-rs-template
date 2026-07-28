# Glory RS

Template para sitios web con **Rust (Axum) + React (TypeScript) + OpenAPI** en un solo repositorio.

Pensado para máxima velocidad de desarrollo, seguridad por defecto y escalabilidad.

## Stack

| Capa                 | Herramienta                  | Para qué                                |
| -------------------- | ---------------------------- | --------------------------------------- |
| Framework web        | Axum                         | HTTP, routing, middleware               |
| OpenAPI              | utoipa + utoipa-swagger-ui   | Genera schema OpenAPI desde código      |
| Serialización        | serde                        | JSON ↔ Structs                          |
| Base de datos        | SQLx (PostgreSQL)            | Queries SQL con verificación            |
| Migraciones          | SQLx migrate                 | Control de schema DB                    |
| Validación           | validator                    | Validar inputs del usuario              |
| Variables de entorno | dotenvy                      | Cargar .env                             |
| Logging              | tracing + tracing-subscriber | Logs estructurados                      |
| Errores              | thiserror                    | Errores tipados                         |
| Auth (JWT)           | jsonwebtoken                 | Tokens                                  |
| Hashing              | argon2                       | Hashing seguro de contraseñas           |
| CORS                 | tower-http                   | Middleware CORS                         |
| Linter               | clippy (paranoia)            | Código limpio                           |
| Frontend             | React + TypeScript + Vite    | UI                                      |
| State management     | React Query + Zustand        | Server state + client state             |
| Codegen              | Orval                        | Genera cliente TypeScript desde OpenAPI |

## Requisitos

- Rust (stable, 1.75+)
- Node.js (18+) y npm
- PostgreSQL corriendo localmente

## Inicio rápido

```bash
# 1. Clonar el template con el framework fijado
git clone --recurse-submodules --branch main https://github.com/1ndoryu/glory-rs-template.git nuevo-proyecto
cd nuevo-proyecto
git submodule update --init --recursive
cp .env.example .env
# Editar .env con tus credenciales de PostgreSQL

# 2. Crear la base de datos
psql -U postgres -c "CREATE DATABASE glory_db;"

# 3. Backend
cargo run
# El servidor inicia en http://localhost:3000
# Swagger UI en http://localhost:3000/swagger-ui/

# 4. Frontend (en otra terminal)
cd frontend
npm install
npm run dev
# Frontend en http://localhost:5173

# 5. Generar cliente API (con backend corriendo)
npm run codegen
```

## Estructura del proyecto

```
├── Cargo.toml              # Dependencias del backend
├── src/
│   ├── main.rs             # Entry point del servidor
│   ├── lib.rs              # Re-exports y AppState
│   ├── config/             # Configuración desde env vars
│   ├── errors/             # Tipos de error → HTTP status codes
│   ├── handlers/           # Capa HTTP (routing, request/response)
│   ├── middleware/          # Auth middleware (JWT extractor)
│   ├── models/             # Structs de dominio y DTOs
│   ├── repositories/       # Capa de base de datos (queries)
│   └── services/           # Lógica de negocio
├── migrations/             # Migraciones SQL (SQLx)
├── frontend/
│   ├── src/
│   │   ├── api/            # Cliente API generado por Orval
│   │   ├── App.tsx         # Componente raíz
│   │   └── main.tsx        # Entry point React
│   ├── orval.config.ts     # Configuración de codegen
│   └── vite.config.ts      # Configuración de Vite + proxy
├── .env.example            # Variables de entorno de ejemplo
└── .gitignore
```

## Arquitectura

El backend sigue separación en capas:

- **handlers/** → Reciben HTTP requests, extraen datos, llaman services, retornan responses
- **services/** → Lógica de negocio, orquestan repositories
- **repositories/** → Queries a PostgreSQL via SQLx
- **models/** → Structs de dominio, DTOs de request/response, schemas OpenAPI
- **errors/** → Enum de errores que mapean a HTTP status codes
- **middleware/** → Extractores de Axum (auth JWT)

## API de ejemplo

El template incluye un CRUD de notas con autenticación:

| Método | Ruta               | Descripción             | Auth |
| ------ | ------------------ | ----------------------- | ---- |
| POST   | /api/auth/register | Registrar usuario       | No   |
| POST   | /api/auth/login    | Iniciar sesión          | No   |
| GET    | /api/health        | Health check            | No   |
| POST   | /api/notes         | Crear nota              | Sí   |
| GET    | /api/notes         | Listar notas (paginado) | Sí   |
| GET    | /api/notes/:id     | Obtener nota            | Sí   |
| PUT    | /api/notes/:id     | Actualizar nota         | Sí   |
| DELETE | /api/notes/:id     | Eliminar nota           | Sí   |

## Ramas por sitio

Este template está diseñado para usar **una rama por sitio/proyecto**:

```bash
git checkout -b mi-sitio-web
# Desarrollar en la rama
# Cambiar a otro sitio:
git checkout otro-sitio
```

La estructura es idéntica en cada rama. Solo cambia el contenido específico del sitio.

## Comandos útiles

```bash
# Comando unificado — verifica todo el proyecto (backend + frontend)
npm run check

# Backend
cargo run                    # Iniciar servidor
cargo check                  # Verificar compilación
cargo clippy                 # Linter (nivel paranoia)
cargo test                   # Tests
cargo fmt                    # Formatear código
npm run check:back           # cargo check + clippy

# Frontend
npm run dev:front            # Dev server con HMR
npm run check:front          # Type-check TypeScript
npm run codegen              # Regenerar cliente API desde OpenAPI

# O directamente desde frontend/
cd frontend
npm run dev                  # Dev server con HMR
npm run build                # Build producción
npm run type-check           # Verificar tipos TypeScript
npm run codegen              # Regenerar cliente API desde OpenAPI
```

## Clippy nivel paranoia

El proyecto tiene configurado clippy en modo estricto (`[lints.clippy]` en Cargo.toml):

- `clippy::all` → **deny** (error en cualquier warning estándar)
- `clippy::pedantic` → **warn** (warnings extra para código idiomático)

Antes de cada commit: `cargo fmt --check && cargo clippy && cargo test`
