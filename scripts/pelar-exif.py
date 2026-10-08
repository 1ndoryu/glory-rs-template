#!/usr/bin/env python3
# [08AA-27] Pela metadatos (EXIF/GPS) de una foto antes de publicarla.
# Aplica la orientación EXIF físicamente (la imagen se sigue viendo igual)
# y guarda limpia, sin ningún metadato. Uso:
#   python scripts/pelar-exif.py entrada.jpg salida.jpg
# Imprime JSON por stdout: {"gps": true|false, "ancho": N, "alto": N}.
# Exit 0 ok | 1 error (mensaje por stderr).
import json
import sys

from PIL import Image, ImageOps


def main():
    if len(sys.argv) != 3:
        print('uso: pelar-exif.py entrada salida', file=sys.stderr)
        return 1
    entrada, salida = sys.argv[1], sys.argv[2]
    try:
        img = Image.open(entrada)
        gps = False
        try:
            exif = img.getexif()
            gps = 34853 in exif  # IFD GPS
        except Exception:
            gps = False
        img = ImageOps.exif_transpose(img)
        params = {}
        if (img.format or '').upper() in ('JPEG', 'JPG'):
            params = {'format': 'JPEG', 'quality': 95}
        elif (img.format or '').upper() == 'PNG':
            params = {'format': 'PNG'}
        elif (img.format or '').upper() == 'WEBP':
            params = {'format': 'WEBP', 'quality': 95}
        else:
            # Por extensión de salida como reserva.
            ext = salida.rsplit('.', 1)[-1].lower()
            params = {'format': {'jpg': 'JPEG', 'jpeg': 'JPEG'}.get(ext, ext.upper())}
        # Sin kwarg exif: PIL guarda sin metadatos.
        img.save(salida, **params)
        print(json.dumps({'gps': gps, 'ancho': img.width, 'alto': img.height}))
        return 0
    except Exception as e:  # Error visible, nunca silencio.
        print(f'FALLO pelar-exif: {e}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
