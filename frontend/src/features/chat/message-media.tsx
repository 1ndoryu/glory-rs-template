/* [299A-3] La burbuja del hilo mostraba `[foto] /uploads/...` y `[audio]
 * ...` como texto plano: el staff no veía la foto ni podía oír la nota de
 * voz. Este componente renderiza esas marcas (las escribe el webhook en
 * `whatsapp.rs:cuerpo_media`, con descripción E11 anexada tras `— se ve:`
 * y transcripción [309A-4] tras `— dice:`).
 * Solo acepta `src` local `/uploads/...` o remoto `http(s)://`; cualquier
 * otra cosa se muestra como texto (nada de `innerHTML`: React escapa).
 * La URL relativa funciona en prod (mismo origen) y en dev vía proxy
 * `/uploads` → :3110 en `vite.config.ts`. */

type MediaLine =
  | { kind: 'text'; text: string }
  | { kind: 'photo'; src: string; caption: string | null }
  | { kind: 'audio'; src: string; caption: string | null }
  | { kind: 'remote'; url: string };

function srcValido(src: string): boolean {
  const s = src.trim();
  return s.startsWith('/uploads/') || s.startsWith('https://') || s.startsWith('http://');
}

function parseLine(linea: string): MediaLine {
  const lineaRecortada = linea.trim();
  if (lineaRecortada.startsWith('[foto] ')) {
    const resto = lineaRecortada.slice('[foto] '.length);
    const [src, ...descripcion] = resto.split(' — se ve: ');
    if (srcValido(src)) {
      const caption = descripcion.length > 0 ? descripcion.join(' — se ve: ').trim() || null : null;
      return { kind: 'photo', src: src.trim(), caption };
    }
  } else if (lineaRecortada.startsWith('[audio] ')) {
    const resto = lineaRecortada.slice('[audio] '.length);
    const [src, ...dicho] = resto.split(' — dice: ');
    if (srcValido(src)) {
      const caption = dicho.length > 0 ? dicho.join(' — dice: ').trim() || null : null;
      return { kind: 'audio', src: src.trim(), caption };
    }
  } else if (lineaRecortada.startsWith('[media] ')) {
    const url = lineaRecortada.slice('[media] '.length).trim();
    if (url.startsWith('https://') || url.startsWith('http://')) return { kind: 'remote', url };
  }
  return { kind: 'text', text: linea };
}

export function MessageMedia({ body }: { body: string }) {
  const lineas = body.split('\n').map(parseLine);
  if (lineas.every((l) => l.kind === 'text')) {
    return <p className="whitespace-pre-wrap">{body}</p>;
  }
  return (
    <div className="space-y-1">
      {lineas.map((linea, i) => {
        if (linea.kind === 'photo') {
          return (
            <figure key={i}>
              <img
                src={linea.src}
                alt="Foto enviada por el visitante"
                loading="lazy"
                className="max-w-full rounded-md"
              />
              {linea.caption && <figcaption className="mt-1 text-xs opacity-70">Se ve: {linea.caption}</figcaption>}
            </figure>
          );
        }
        if (linea.kind === 'audio') {
          return (
            <figure key={i}>
              <audio controls preload="none" src={linea.src} className="w-full max-w-65">
                Tu navegador no reproduce este audio.
              </audio>
              {linea.caption && <figcaption className="mt-1 text-xs opacity-70">Dice: {linea.caption}</figcaption>}
            </figure>
          );
        }
        if (linea.kind === 'remote') {
          return (
            <p key={i}>
              <a href={linea.url} target="_blank" rel="noreferrer" className="underline">
                Abrir archivo adjunto
              </a>
            </p>
          );
        }
        return (
          <p key={i} className="whitespace-pre-wrap">
            {linea.text}
          </p>
        );
      })}
    </div>
  );
}
