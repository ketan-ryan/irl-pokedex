import json

# Load JSONs
with open("pokemon_home_images_full.json") as f:
    home_images = json.load(f)

with open("pokemon_images.json") as f:
    db_images = json.load(f)

master_json = {}

# Step 1: Add first 898 Pokémon from home_images, include name with Dex prefix
for dex_str, forms in home_images.items():
    dex_int = int(dex_str)
    # Get Pokémon name from db_images in the same order (sorted by Dex)
    name = list(db_images.keys())[dex_int - 1]  # Dex 1 -> index 0
    # Add Dex-prefixed name
    master_json[dex_str] = {
        "name": f"{dex_int:04d}{name}",
        **forms
    }

# Step 2: Add remaining Pokémon (Dex 899+)
for dex_int, name in enumerate(list(db_images.keys())[898:], start=899):
    dex_str = str(dex_int)
    image_url = db_images[name]["image_url"]
    master_json[dex_str] = {
        "name": f"{dex_int:04d}{name}",
        "base": {
            "default": {
                "normal": image_url,
                "shiny": None
            }
        }
    }

# Save master JSON
with open("pokemon_master.json", "w") as f:
    json.dump(master_json, f, indent=4)

print(f"Master JSON created with {len(master_json)} Pokémon entries.")