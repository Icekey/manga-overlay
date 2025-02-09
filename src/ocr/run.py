from pathlib import Path

from PIL import Image
from PIL import UnidentifiedImageError
from manga_ocr import MangaOcr

MANGA_OCR = MangaOcr(pretrained_model_name_or_path='kha-white/manga-ocr-base',
                     force_cpu=False)


def get_images_ocr(paths):
    images = get_images(paths)

    texts = MANGA_OCR(images)

    return texts


def get_images(paths):
    images = []
    for path in paths:
        read_from = Path(path)
        if not read_from.is_file():
            print(f'{path} is not a file')
            continue
        try:
            img = Image.open(read_from)
            img.load()
            images.append(img)
        except (UnidentifiedImageError, OSError) as e:
            print(f'Error while reading file {read_from}: {e}')
    return images
