export interface ReaderPreviewOptions {
  title: string;
}

function createParagraph(text: string): HTMLParagraphElement {
  const paragraph = document.createElement('p');
  paragraph.textContent = text;
  return paragraph;
}

function createArticleImage(src: string, alt: string): HTMLImageElement {
  const image = document.createElement('img');
  image.className = 'desktop-reader__image';
  image.src = src;
  image.alt = alt;
  image.loading = 'lazy';
  return image;
}

/* [297A-2] Preview seguro del lector: construye el artículo con DOM y texto,
 * sin innerHTML; el renderer TipTap sanitizado reemplazará estos datos en fase 3. */
export function createReaderPreview(options: ReaderPreviewOptions): HTMLElement {
  const article = document.createElement('article');
  article.className = 'desktop-reader';

  const header = document.createElement('header');
  header.className = 'desktop-reader__header';

  const title = document.createElement('h1');
  title.className = 'desktop-reader__title';
  title.textContent = options.title;

  const date = document.createElement('time');
  date.className = 'desktop-reader__date';
  date.dateTime = '2026-07-15';
  date.textContent = '15 julio 2026';
  header.append(title, date);

  article.append(
    header,
    createArticleImage(
      '/legacy-assets/colors/0e258aa6aca4067b3402bc82aac0798f.jpg',
      'Abstracción de color naranja y rosa',
    ),
    createParagraph('Hay algo en el ruido blanco de los servidores que me recuerda al mar. No el mar turístico de postal, sino el otro: el que nadie ve de madrugada cuando la ciudad duerme.'),
    createParagraph('Escribí este texto pensando en cómo las máquinas tienen su propio silencio, y cómo ese silencio a veces dice más que cualquier palabra.'),
  );

  const quote = document.createElement('blockquote');
  quote.className = 'desktop-reader__quote';
  quote.textContent = 'Una interfaz también puede ser un lugar: se entra, se dejan cosas abiertas y se vuelve después.';
  article.append(
    quote,
    createArticleImage(
      '/legacy-assets/colors/ea78d06a9fce9a2a10d3f7e3f6eb9314.png',
      'Abstracción azul, cian y coral',
    ),
    createParagraph('Por eso este sitio empieza a parecerse menos a una página y más a un escritorio. Los artículos son documentos; las imágenes, objetos que uno encuentra dentro de una carpeta.'),
  );

  return article;
}
