import { defineConfig } from 'orval';

export default defineConfig({
  glory: {
    input: {
      /* Apuntar al backend corriendo localmente para obtener el schema OpenAPI */
      target: 'http://localhost:3000/api-docs/openapi.json',
    },
    output: {
      /* Vanilla TS usa el cliente fetch compartido; tags-split evita un
       * generated.ts monolítico y permite regenerar por dominio. */
      target: './src/api/generated/index.ts',
      client: 'fetch',
      mode: 'tags-split',
    },
  },
});
