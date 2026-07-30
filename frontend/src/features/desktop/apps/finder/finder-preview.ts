import { FileText, createElement } from 'lucide';

export interface FinderPreviewOptions {
  onOpenArticle: (title: string) => void;
}

const folderImages = [
  {
    src: '/legacy-assets/colors/0e258aa6aca4067b3402bc82aac0798f.jpg',
    label: 'color_01.jpg',
    alt: 'Abstracción de color naranja y rosa',
  },
  {
    src: '/legacy-assets/colors/c5f3015667280079a5a6299c0ac16e83.jpg',
    label: 'flores_02.jpg',
    alt: 'Flores y sombras rosadas',
  },
  {
    src: '/legacy-assets/colors/ea78d06a9fce9a2a10d3f7e3f6eb9314.png',
    label: 'forma_03.png',
    alt: 'Abstracción azul, cian y coral',
  },
];

const folderDocuments = [
  'El silencio de las máquinas',
  'Fragmentos de código',
];

/* [297A-2] Preview Finder: entrega solo contenido; DesktopWindow aporta todo el chrome.
 * Las imágenes son demostrativas y se sustituirán por el servicio de media en fase 3. */
export function createFinderPreview(options: FinderPreviewOptions): HTMLElement {
  const finder = document.createElement('div');
  finder.className = 'desktop-finder';

  const path = document.createElement('div');
  path.className = 'desktop-finder__path';
  path.textContent = 'Galería / julio 2026';

  const grid = document.createElement('div');
  grid.className = 'desktop-finder__grid';

  for (const imageData of folderImages) {
    const item = document.createElement('figure');
    item.className = 'desktop-finder__item desktop-finder__item--image';

    const image = document.createElement('img');
    image.className = 'desktop-finder__thumbnail';
    image.src = imageData.src;
    image.alt = imageData.alt;
    image.loading = 'lazy';

    const label = document.createElement('figcaption');
    label.className = 'desktop-finder__label';
    label.textContent = imageData.label;
    item.append(image, label);
    grid.appendChild(item);
  }

  for (const title of folderDocuments) {
    const item = document.createElement('button');
    item.type = 'button';
    item.className = 'desktop-finder__item desktop-finder__item--document';
    item.setAttribute('aria-label', `Abrir artículo ${title}`);

    const icon = createElement(FileText);
    icon.classList.add('desktop-finder__document-icon');

    const label = document.createElement('span');
    label.className = 'desktop-finder__label';
    label.textContent = title;

    item.append(icon, label);
    item.addEventListener('click', () => options.onOpenArticle(title));
    grid.appendChild(item);
  }

  finder.append(path, grid);
  return finder;
}
