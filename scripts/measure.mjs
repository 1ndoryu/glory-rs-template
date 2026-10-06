/* [03AA-3 M3] Mide p50/p95 del borrador mp.
 * Mock (defecto, N=50): costo local de armar+validar el schema espejo (sin red
 * ni BD): detecta regresiones del contrato, no del servidor.
 * --real (opt-in, N=10): POST /api/admin/marketplace/borrador contra el
 * backend local con MP_TOKEN (JWT mp); exige p95 < 8000 ms (DoD del plan).
 */
const HEX64 = /^[0-9a-f]{64}$/;

function pedido(i) {
  return {
    threadId: `hilo-medicion-${i}`,
    firma: 'ab'.repeat(32),
    firma_version: 'firma-v1',
    lang: 'es',
    excerpt: {
      remitente_hash: 'cd'.repeat(32),
      texto: 'Hola, ¿sigue disponible?',
      hora: '2026-10-05T18:00:00-04:00',
      leido: true,
    },
    avisoId: null,
    extras: { tono: 'amable', largo: 's' },
  };
}

function validarLocal(r) {
  const e = [];
  if (!r.threadId) e.push('threadId');
  if (!HEX64.test(r.firma)) e.push('firma');
  if (r.firma_version !== 'firma-v1') e.push('firma_version');
  if (!/^[a-z]{2}$/.test(r.lang)) e.push('lang');
  if (!r.excerpt.hora.endsWith('-04:00')) e.push('hora');
  if (e.length) throw new Error('schema espejo roto: ' + e.join(','));
  return JSON.stringify(r).length;
}

function percentiles(ms) {
  const v = [...ms].sort((a, b) => a - b);
  const q = (p) => v[Math.min(v.length - 1, Math.floor(p * v.length))];
  return { p50: q(0.5), p95: q(0.95), n: v.length };
}

async function mock(n) {
  const ms = [];
  for (let i = 0; i < n; i++) {
    const t0 = performance.now();
    validarLocal(pedido(i));
    ms.push(performance.now() - t0);
  }
  return percentiles(ms);
}

async function real(n) {
  const token = process.env.MP_TOKEN;
  if (!token) {
    console.error('MP_TOKEN sin definir (JWT mp de /api/admin/marketplace/token)');
    process.exit(2);
  }
  const ms = [];
  for (let i = 0; i < n; i++) {
    const t0 = performance.now();
    const r = await fetch('http://127.0.0.1:3110/api/admin/marketplace/borrador', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
      body: JSON.stringify(pedido(i)),
    });
    if (!r.ok) {
      console.error(`iter ${i}: HTTP ${r.status} ${(await r.text()).slice(0, 200)}`);
      process.exit(1);
    }
    ms.push(performance.now() - t0);
  }
  return percentiles(ms);
}

const args = process.argv.slice(2);
const modoReal = args.includes('--real');
const n = modoReal ? 10 : 50;
const res = await (modoReal ? real(n) : mock(n));
console.log(`${modoReal ? 'REAL' : 'MOCK'} p50=${res.p50.toFixed(2)}ms p95=${res.p95.toFixed(2)}ms n=${res.n}`);
if (modoReal && res.p95 >= 8000) {
  console.error('DoD incumplido: p95 >= 8000 ms');
  process.exit(1);
}
