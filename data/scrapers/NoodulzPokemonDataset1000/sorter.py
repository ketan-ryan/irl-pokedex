import os
import json
import re

# CONFIG
POKEMON_JSON = "../all_pokemon_safe.json"   # ordered list of pokemon names
TARGET_DIR = r"G:\My Drive\IRL Pokedex\128x128 sprites"
PAD_WIDTH = 4
# ------------------------

def is_already_numbered(folder_name):
    return bool(re.match(r"^\d{4}", folder_name))

with open(POKEMON_JSON, "r", encoding="utf-8") as f:
    pokemon_names = json.load(f)

# normalize pokemon names to match existing case
pokemon_names_normalized = [
    name
        .lower()
        .replace(" ", "-")
        .replace(".", "")
        .replace("'", "")
    for name in pokemon_names
]

folders = [
    name for name in os.listdir(TARGET_DIR)
    if os.path.isdir(os.path.join(TARGET_DIR, name))
]

for idx, pokemon in enumerate(pokemon_names_normalized, start=1):
    padded_number = str(idx).zfill(PAD_WIDTH)
    pokemon_lower = pokemon.lower()

    # Ignore folders already numbered
    candidate_folders = [
        f for f in folders if not is_already_numbered(f)
    ]

    # 1. Exact match (case-insensitive)
    exact_matches = [
        f for f in candidate_folders
        if f.lower() == pokemon_lower
    ]
    if len(exact_matches) == 1:
        match = exact_matches[0]
    else:
        # 2. Fallback: name contained
        contains_matches = [
            f for f in candidate_folders
            if pokemon_lower in f.lower()
        ]

        if len(contains_matches) == 1:
            match = contains_matches[0]
        elif len(contains_matches) == 0:
            print(f"[WARN] No folder found for {pokemon}")
            continue
        else:
            print(f"[WARN] Multiple folders found for {pokemon}: {contains_matches}")
            continue

    old_name = match
    new_name = f"{padded_number}{old_name}"

    old_path = os.path.join(TARGET_DIR, old_name)
    new_path = os.path.join(TARGET_DIR, new_name)

    print(f"Renaming: {old_name} -> {new_name}")
    os.rename(old_path, new_path)

    # Update folder list so future matches see the new name
    folders.remove(old_name)
    folders.append(new_name)
