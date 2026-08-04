# Glory RS / wandori.us

Template y aplicación web con **Rust (Axum) + PostgreSQL + Vanilla TypeScript/Vite + OpenAPI** en un solo repositorio.

Pensado para velocidad de desarrollo, seguridad por defecto y calidad reproducible. El quality gate unificado usa `scripts/quality/task-check.mjs` como orquestador de transición, Sentinel como etapa/analyzer y VarSense como analizador especializado; el runtime global de Sentinel aún no forma parte de este checkout.

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
| Auth (sesión opaca)  | cookie HttpOnly + CSRF       | Sesiones revocables                      |
| Hashing              | argon2                       | Hashing seguro de contraseñas           |
| CORS                 | tower-http                   | Middleware CORS                         |
| Linter               | clippy (paranoia)            | Código limpio                           |
| Frontend             | Vanilla TypeScript + Vite   | UI del OS retro y apps                 |
| Estado               | Stores/adapters propios      | Estado runtime, sesión y workspace     |
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
│   ├── middleware/          # Auth middleware (sesion opaca HttpOnly)
│   ├── models/             # Structs de dominio y DTOs
│   ├── repositories/       # Capa de base de datos (queries)
│   └── services/           # Lógica de negocio
├── migrations/             # Migraciones SQL (SQLx)
├── frontend/
│   ├── src/
│   │   ├── api/            # Cliente API generado por Orval
│   │   ├── components/     # UI compartida
│   │   ├── features/       # Runtime del OS y apps
│   │   ├── pages/          # Vistas y adaptadores de ruta
│   │   └── main.ts         # Entry point Vanilla TypeScript
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
- **middleware/** → Extractores de Axum (sesion opaca HttpOnly y CSRF)

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

## Calidad y comandos de desarrollo

El comando público de validación es el gate único. Decide el alcance por los
archivos modificados, conserva los resultados por rama y escribe el detalle en
`.quality-reports/branches/<branch-key>/<task-id>/`.

```bash
# Gate local incremental; el ID debe existir en roadmap/planes/completados
npm run task:check -- 028A-6

# Gate completo para cierre de fase o CI (no repetir durante el cooldown)
npm run task:check -- 028A-6 --full
npm run task:check -- 028A-6 --ci

# Contratos y diagnóstico del stack de calidad
npm run quality:test
npm run quality:doctor
npm run quality:lock -- --check
npm run quality:reports:cleanup:dry
```

`sentinel.lock.json` fija las versiones, commits, capacidades, protocolos y
hashes de los analizadores. El gate consume los `main` externos mediante las
variables `GLORY_SENTINEL_SOURCE_PATH` y `GLORY_VARSENSE_SOURCE_PATH`; no guarda
rutas absolutas en el repositorio. En la transición actual el runtime se declara
`project-adapter` y `artifactSha256: null`; no se instala un runtime global ni se
ejecuta código arbitrario desde la política del proyecto.

Antes de `quality:lock` o del gate, define las rutas locales a los checkouts
publicados y limpios:

```bash
export GLORY_SENTINEL_SOURCE_PATH=/ruta/al/glory-sentinel
export GLORY_VARSENSE_SOURCE_PATH=/ruta/al/varsense
npm run quality:lock -- --check
```

En PowerShell usa `$env:GLORY_SENTINEL_SOURCE_PATH` y
`$env:GLORY_VARSENSE_SOURCE_PATH`. El preflight comprueba `realpath`, Git, CLI,
versión, commit, hash de archive y que el `realpath` resuelto siga
apuntando al checkout actual sin persistir esa ruta en el lock.

Los wrappers de desarrollo (`npm run check:back`, `npm run check:front`,
`npm run fmt:check` y `npm test`) siguen disponibles para trabajo específico,
pero no sustituyen el reporte ni el control del gate. Para una validación que
pueda cerrar una tarea, usa `task:check` desde la raíz del repositorio.

### Desarrollo

```bash
npm run dev                  # Backend + contexto de desarrollo por rama
npm run dev:front            # Frontend con HMR
npm run codegen              # Regenerar cliente API desde OpenAPI
cd frontend && npm run build # Build frontend explícito
```

## Clippy nivel paranoia

El proyecto tiene configurado clippy en modo estricto (`[lints.clippy]` en Cargo.toml):

- `clippy::all` → **deny** (error en cualquier warning estándar)
- `clippy::pedantic` → **warn** (warnings extra para código idiomático)

Antes de cerrar una tarea: `npm run task:check -- <ID>`; para una fase o publicación, repetir con `--full` o `--ci`. El gate deriva la base de datos/contexto por rama cuando una etapa Rust lo necesita.
