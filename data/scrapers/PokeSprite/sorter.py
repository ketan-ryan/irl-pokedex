import shutil
from pathlib import Path
import json

# Files
INPUT_FILE = Path("pokemon_sprites.txt")
DEST_ROOT = Path("Sprites")
POKEMON_JSON = Path("pokesprite/data/pokemon.json")

# Load JSON to get canonical slugs
with open(POKEMON_JSON, "r", encoding="utf-8") as f:
    data = json.load(f)

# Map JSON slugs for quick lookup
slug_set = set(info["slug"]["eng"] for info in data.values())

# Create root folder if it doesn't exist
DEST_ROOT.mkdir(exist_ok=True)

# Go through each path in the TXT file
with open(INPUT_FILE, "r", encoding="utf-8") as f:
    for line in f:
        path_str = line.strip()
        src_path = Path(path_str)

        # Determine tags from folder structure
        tags = []
        if "shiny" in src_path.parts:
            tags.append("shiny")
        if "female" in src_path.parts:
            tags.append("female")
        if "right" in src_path.parts:
            tags.append("right")

        # Parse filename to get slug and form
        stem = src_path.stem  # filename without extension

        # Split tags first (after underscores)
        parts = stem.split("_")
        base_name = parts[0]

        # Determine base slug by matching JSON slugs
        matched_slug = None
        form = ""
        for slug in slug_set:
            if base_name == slug:
                matched_slug = slug
                break
            elif base_name.startswith(slug + "-"):
                matched_slug = slug
                form = base_name[len(slug)+1:]  # rest after slug-
                break

        if not matched_slug:
            # fallback if slug not found in JSON
            matched_slug = base_name

        # Destination folder: Sprites/<matched_slug>
        dest_folder = DEST_ROOT / matched_slug
        dest_folder.mkdir(exist_ok=True)

        # Build new filename
        new_name = matched_slug
        if form:
            new_name += f"-{form}"
        if tags:
            new_name += "_" + "_".join(tags)
        new_name += src_path.suffix  # keep .png

        dest_path = dest_folder / new_name

        # Copy file
        shutil.copy2(src_path, dest_path)