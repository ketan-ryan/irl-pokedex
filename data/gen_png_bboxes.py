"""
Usage:
python gen_png_bboxes.py "image_dir" [optional] --visualize
"""

import argparse
import json
import re
import shutil
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from tqdm import tqdm
import yaml

def find_bounding_box(img_array: np.ndarray, min_alpha: int = 1) -> dict | None:
    """
    Given an RGBA image as a NumPy array (H x W x 4),
    return the bounding box of all non-transparent pixels.

    Returns a dict with keys: x, y, width, height, x2, y2
    Returns None if the image is fully transparent.
    """
    alpha = img_array[:, :, 3]  # Extract alpha channel
    mask = alpha >= min_alpha   # Boolean mask: True where pixel is visible

    # Find rows and columns that contain at least one visible pixel
    rows_with_content = np.any(mask, axis=1)  # shape: (H,)
    cols_with_content = np.any(mask, axis=0)  # shape: (W,)

    if not rows_with_content.any():
        return None  # Fully transparent image

    # argmax on a boolean array gives the index of the first True
    top    = int(np.argmax(rows_with_content))
    bottom = int(len(rows_with_content) - 1 - np.argmax(rows_with_content[::-1]))
    left   = int(np.argmax(cols_with_content))
    right  = int(len(cols_with_content) - 1 - np.argmax(cols_with_content[::-1]))

    return {
        "x":      left,
        "y":      top,
        "x2":     right,
        "y2":     bottom,
        "width":  right - left + 1,
        "height": bottom - top + 1,
    }


def process_image(path: Path, classes: list[str], viz_path: Path, min_alpha: int = 1, visualize: bool = False) -> dict:
    """Process a single PNG and return its bounding box info."""
    img = Image.open(path).convert("RGBA")
    img_array = np.array(img)

    h, w = img_array.shape[:2]
    bbox = find_bounding_box(img_array, min_alpha=min_alpha)

    result = {
        "file":           str(path.name),
        "image_width":    w,
        "image_height":   h,
        "bounding_box":   None,
    }

    if bbox is None:
        print(f"[SKIP] {path.name} — fully transparent")
        return result

    center_x = ((bbox["x"] + bbox["x2"]) / 2.0) / w
    center_y = ((bbox["y"] + bbox["y2"]) / 2.0) / h
    width = (bbox["x2"] - bbox["x"]) / w
    height = (bbox["x2"] - bbox["x"]) / h

    parent = str(path.parent.name)
    # remove leading numbers
    name_only = re.sub(r"^\d+", "", parent)
    if name_only not in classes:
        print(f'WARN: {name_only} not found in classes!')
        return result
    
    class_id = classes.index(name_only)

    result = {
        'file': str(path.stem),
        'center_x': center_x,
        'center_y': center_y,
        'width': width,
        'height': height,
        'class_id': class_id
    }

    if visualize:
        draw = ImageDraw.Draw(img)
        draw.rectangle(
            [bbox["x"], bbox["y"], bbox["x2"], bbox["y2"]],
            outline=(255, 0, 0, 255),
            width=2,
        )
        vis_path = path.with_stem(path.stem + "_bbox")
        img.save(viz_path.joinpath(vis_path.name))
        result["visualized_file"] = str(vis_path)

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Generate bounding boxes for PNG images using numpy"
    )
    parser.add_argument("input_path", help="Directory of PNGs")
    parser.add_argument("--visualize", action="store_true",
                        help="Save a copy of each image with the bounding box drawn")
    args = parser.parse_args()

    input_path = Path(args.input_path)

    classes = []
    with open('all_pokemon_safe.json', 'r') as fp:
        classes = json.load(fp)

    classes_lower = [c.lower() for c in classes]

    if input_path.is_dir():
        png_files = sorted(
            p for p in input_path.rglob("*.png")
            if "exclude" not in p.parts
            and "visualizations" not in p.parts
        )
        if not png_files:
            print(f"No PNG files found in {input_path}")
            sys.exit(1)
    else:
        print(f"Input must be a directory. Got: {input_path}")
        sys.exit(1)

    output_path = Path(input_path.joinpath('annotations'))
    if output_path.exists(): shutil.rmtree(output_path)

    output_path.mkdir(exist_ok=True)

    viz_path = output_path.joinpath('visualizations')
    viz_path.mkdir(exist_ok=True)

    res_path = output_path.joinpath('labels/train')
    res_path.mkdir(parents=True, exist_ok=True)

    results: list[dict] = []
    for png in tqdm(png_files):
        result = process_image(
            png,
            visualize=args.visualize,
            classes=classes_lower,
            viz_path=viz_path
        )
        if 'center_x' not in result.keys(): continue
        results.append(result)

        with open(f'{res_path}/{result["file"]}.txt', 'w') as fp:
            fp.write(f"{result['class_id']} {result['center_x']} {result['center_y']} {result['width']} {result['height']}\n")

    data = {
        'names': dict(enumerate(classes)),
        'path': '.',
        'train': 'train.txt'
    }
    with open(f'{output_path}/data.yaml', 'w') as fp:
        yaml.dump(data, fp)
        
    with open(f'{output_path}/train.txt', "w") as fp:
        for res in results:
            if 'center_x' not in res.keys():
                continue
                
            fp.write(f'data/images/train/{res['file']}.png\n')


    found = sum(1 for r in results if r["class_id"] is not None)
    print(f"\nDone. {found}/{len(results)} images had content.")
    print(f"Results saved to: {output_path}")


if __name__ == "__main__":
    main()