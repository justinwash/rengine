#!/usr/bin/env python3
"""Center-crop images to a square and resize them in bulk."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageOps, UnidentifiedImageError

SUPPORTED_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp", ".tif", ".tiff"}

try:
    RESAMPLE = Image.Resampling.LANCZOS
except AttributeError:
    RESAMPLE = Image.LANCZOS


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Center-crop images to a square and resize them."
    )
    parser.add_argument("input_dir", type=Path, help="Folder containing source images.")
    parser.add_argument(
        "-o",
        "--output",
        dest="output_dir",
        type=Path,
        help="Folder for processed images. Defaults to <input>/resized_<size>.",
    )
    parser.add_argument(
        "-s",
        "--size",
        type=int,
        default=512,
        help="Output width and height in pixels. Default: 512.",
    )
    parser.add_argument(
        "-r",
        "--recursive",
        action="store_true",
        help="Process images recursively and preserve the folder structure.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite files that already exist in the output folder.",
    )
    return parser.parse_args()


def iter_images(input_dir: Path, recursive: bool):
    paths = input_dir.rglob("*") if recursive else input_dir.iterdir()
    for path in paths:
        if path.is_file() and path.suffix.lower() in SUPPORTED_EXTENSIONS:
            yield path


def build_output_path(src: Path, input_dir: Path, output_dir: Path, recursive: bool) -> Path:
    relative_path = src.relative_to(input_dir) if recursive else Path(src.name)
    return output_dir / relative_path


def save_resized_square(src: Path, dst: Path, size: int) -> None:
    with Image.open(src) as original:
        image = ImageOps.exif_transpose(original)
        resized = ImageOps.fit(image, (size, size), method=RESAMPLE, centering=(0.5, 0.5))

        output_image = resized
        if dst.suffix.lower() in {".jpg", ".jpeg"} and output_image.mode not in {"RGB", "L"}:
            output_image = output_image.convert("RGB")

        dst.parent.mkdir(parents=True, exist_ok=True)
        save_kwargs = {}
        if dst.suffix.lower() in {".jpg", ".jpeg"}:
            save_kwargs.update({"quality": 95, "optimize": True})
        elif dst.suffix.lower() == ".png":
            save_kwargs["optimize"] = True

        output_image.save(dst, **save_kwargs)


def main() -> int:
    args = parse_args()
    input_dir = args.input_dir.expanduser().resolve()
    if not input_dir.is_dir():
        raise SystemExit(f"Input directory not found: {input_dir}")

    output_dir = (args.output_dir or input_dir / f"resized_{args.size}").expanduser().resolve()
    if output_dir == input_dir:
        raise SystemExit("Output directory must be different from the input directory.")

    sources = list(iter_images(input_dir, args.recursive))
    if not sources:
        print(f"No supported images found in {input_dir}")
        return 0

    output_dir.mkdir(parents=True, exist_ok=True)

    processed = 0
    skipped = 0

    for src in sources:
        dst = build_output_path(src, input_dir, output_dir, args.recursive)
        if dst.exists() and not args.overwrite:
            skipped += 1
            print(f"skip {src} -> {dst} (already exists)")
            continue

        try:
            save_resized_square(src, dst, args.size)
            processed += 1
            print(f"ok   {src} -> {dst}")
        except UnidentifiedImageError:
            skipped += 1
            print(f"skip {src} (unrecognized image file)")
        except OSError as exc:
            skipped += 1
            print(f"skip {src} ({exc})")

    print(f"Processed {processed} image(s); skipped {skipped}. Output: {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())