import os
import json
import shutil
import re

# ---------------- CONFIG ---------------- #
SOURCE_DIR = r"C:\Users\varen\Desktop\poke model\sugimori sprites\Sugimori Pokémon Gen1-9 DLC3 Organized\Sugimori Pokémon Gen1-9 DLC3 Organized"
OUTPUT_DIR = r"G:\My Drive\IRL Pokedex\sugimora sprites"
POKEMON_JSON = "all_pokemon.json"

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp", ".tiff"}
# ---------------------------------------- #

def normalize_name(name):
    """Lowercase + remove non-alphanumeric for matching"""
    return re.sub(r"[^a-z0-9]", "", name.lower())


def is_image_file(filename):
    return os.path.splitext(filename)[1].lower() in IMAGE_EXTENSIONS


def load_pokemon_data(json_path):
    """
    Returns:
    - pokemon_lookup: normalized_name -> folder_name (e.g. 0032nidoran)
    """
    with open(json_path, "r", encoding="utf-8") as f:
        names = json.load(f)

    pokemon_lookup = {}

    for idx, name in enumerate(names, start=1):
        dex_num = f"{idx:04d}"
        folder_name = f"{dex_num}{normalize_name(name)}"
        pokemon_lookup[normalize_name(name)] = folder_name

    return pokemon_lookup


def main():
    pokemon_lookup = load_pokemon_data(POKEMON_JSON)

    # Create all Pokémon folders (1025 total)
    for folder in pokemon_lookup.values():
        os.makedirs(os.path.join(OUTPUT_DIR, folder), exist_ok=True)

    unmatched = []

    for root, _, files in os.walk(SOURCE_DIR):
        for file in files:
            if not is_image_file(file):
                continue

            src_path = os.path.join(root, file)
            file_norm = normalize_name(file)

            matched = False

            for norm_name, folder_name in pokemon_lookup.items():
                if norm_name in file_norm:
                    dest_dir = os.path.join(OUTPUT_DIR, folder_name)
                    os.makedirs(dest_dir, exist_ok=True)

                    dest_path = os.path.join(dest_dir, file)

                    # Prevent overwrite
                    if os.path.exists(dest_path):
                        base, ext = os.path.splitext(file)
                        i = 1
                        while os.path.exists(dest_path):
                            dest_path = os.path.join(
                                dest_dir, f"{base}_{i}{ext}"
                            )
                            i += 1

                    shutil.copy2(src_path, dest_path)
                    matched = True
                    break

            if not matched:
                unmatched.append(src_path)

    print("Done organizing Pokémon images.")
    print(f"Unmatched images: {len(unmatched)}")

    if unmatched:
        with open("unmatched_images.txt", "w", encoding="utf-8") as f:
            for path in unmatched:
                f.write(path + "\n")
        print("Unmatched image paths saved to unmatched_images.txt")


if __name__ == "__main__":
    main()