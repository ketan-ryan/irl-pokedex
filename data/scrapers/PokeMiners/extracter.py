import os
import re
import json
import shutil

# CONFIG
IMAGE_DIR = r"PokeMiners pogo_assets master Images-Pokemon_Addressable Assets"
# IMAGE_DIR = r"PokeMiners pogo_assets master Images-Pokemon - 256x256"  #UNCOMMENT THIS
OUTPUT_DIR = r"G:\My Drive\IRL Pokedex\go models"

# Load Pokémon names
try:
    with open('all_pokemon_safe.json', 'r') as file:
        pokemon_names = json.load(file)
except FileNotFoundError:
    pokemon_names = []
    print("Error: The file was not found.")

# Regex to extract pokedex number
pattern_256x256 = re.compile(r"pokemon_icon_(?:pm)?0*(\d{1,3})_\d+")
pattern_addressable_assets = re.compile(r"^pm0*(\d+)\.")

os.makedirs(OUTPUT_DIR, exist_ok=True)

# Track which source files we have copied
copied_files = set()

# Process images
for filename in os.listdir(IMAGE_DIR):
    src = os.path.join(IMAGE_DIR, filename)
    if not os.path.isfile(src):
        continue

    if filename in copied_files:
        continue  # already copied this source file


    match = pattern_256x256.search(filename)
    if not match:
        match = pattern_addressable_assets.search(filename)
    if not match:
        print(f"couldn't recognize: {filename}")
        continue

    dex_number = int(match.group(1))          # 001 → 1
    list_index = dex_number - 1               # 0-based index

    if list_index < 0 or list_index >= len(pokemon_names):
        print(f"Skipping unknown dex #{dex_number}")
        continue

    pokemon_name = pokemon_names[list_index]
    folder_name = f"{dex_number:04d}{pokemon_name}"
    folder_path = os.path.join(OUTPUT_DIR, folder_name)
    os.makedirs(folder_path, exist_ok=True)

    dst = os.path.join(folder_path, filename)

    # copy only if the file doesn't already exist in the folder
    if not os.path.exists(dst):
        shutil.copy(src, dst)
        copied_files.add(filename)

print("Sorting complete!")