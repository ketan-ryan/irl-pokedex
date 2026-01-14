import json
from pathlib import Path

BASE = Path("pokesprite")
GEN7 = BASE / "pokemon-gen7x"
GEN8 = BASE / "pokemon-gen8"
OUTPUT_FILE = Path("pokemon_sprites.txt")

with open(BASE / "data" / "pokemon.json", "r", encoding="utf-8") as f:
    data = json.load(f)

unique_pokemon = set()  # To store first part of slug for uniqueness
written_paths = set()

# Open the file in write mode (overwrites if exists)
with open(OUTPUT_FILE, "w", encoding="utf-8") as out_file:

    for dex, info in data.items():
        slug = info["slug"]["eng"]

        # Add the base slug to the set
        unique_pokemon.add(slug)

        gen8_data = info.get("gen-8", {})
        gen8_forms = gen8_data.get("forms", {})

        for form, flags in gen8_forms.items():

            # Skip alias forms
            if "is_alias_of" in flags:
                continue

            # CASE 1: Gen 8 sprite exists normally
            if not flags.get("is_prev_gen_icon", False):

                if form == "$":
                    base_reg = GEN8 / "regular" / f"{slug}.png"
                    base_shiny = GEN8 / "shiny" / f"{slug}.png"
                    female_reg = GEN8 / "regular" / "female" / f"{slug}.png"
                    female_shiny = GEN8 / "shiny" / "female" / f"{slug}.png"
                else:
                    base_reg = GEN8 / "regular" / f"{slug}-{form}.png"
                    base_shiny = GEN8 / "shiny" / f"{slug}-{form}.png"
                    female_reg = GEN8 / "regular" / "female" / f"{slug}-{form}.png"
                    female_shiny = GEN8 / "shiny" / "female" / f"{slug}-{form}.png"

                # Print missing sprites
                if not base_reg.exists():
                    print(f"MISSING pokemon from GEN8={slug} form={form}")

                if not base_shiny.exists():
                    print(f"MISSING SHINY pokemon from GEN8={slug} form={form}")

                if flags.get("has_female") and not female_reg.exists():
                    print(f"MISSING GEN8 FEMALE={female_reg} form={form}")
            
                # Write existing paths to file
                for path in [base_reg, base_shiny, female_reg, female_shiny]:
                    if path.exists() and str(path) not in written_paths:
                        out_file.write(str(path) + "\n")
                        written_paths.add(str(path))

            # CASE 2: Gen 8 uses Gen 7 sprite
            else:
                gen7_data = info.get("gen-7", {})
                gen7_forms = gen7_data.get("forms", {})

                for g7_form, g7_flags in gen7_forms.items():

                    if "is_alias_of" in g7_flags:
                        continue

                    if g7_form == "$":
                        base_reg = GEN7 / "regular" / f"{slug}.png"
                        base_shiny = GEN7 / "shiny" / f"{slug}.png"
                        female_reg = GEN7 / "regular" / "female" / f"{slug}.png"
                        female_shiny = GEN7 / "shiny" / "female" / f"{slug}.png"
                        right_reg = GEN7 / "regular" / "right" / f"{slug}.png"
                        right_shiny = GEN7 / "shiny" / "right" / f"{slug}.png"
                    else:
                        base_reg = GEN7 / "regular" / f"{slug}-{g7_form}.png"
                        base_shiny = GEN7 / "shiny" / f"{slug}-{g7_form}.png"
                        female_reg = GEN7 / "regular" / "female" / f"{slug}-{g7_form}.png"
                        female_shiny = GEN7 / "shiny" / "female" / f"{slug}-{g7_form}.png"
                        right_reg = GEN7 / "regular" / "right" / f"{slug}-{g7_form}.png"
                        right_shiny = GEN7 / "shiny" / "right" / f"{slug}-{g7_form}.png"

                    # Print missing sprites
                    if not base_reg.exists():
                        print(f"MISSING pokemon from GEN7={slug} form={g7_form}")

                    if not base_shiny.exists():
                        print(f"MISSING SHINY pokemon from GEN7={slug} form={g7_form}")

                    if g7_flags.get("has_female") and not female_reg.exists():
                        print(f"MISSING GEN7 FEMALE={female_reg} form={g7_form}")

                    if g7_flags.get("has_right") and not right_reg.exists():
                        print(f"MISSING GEN7 RIGHT={right_reg} form={g7_form}")

                    # Write existing paths to file
                    for path in [base_reg, base_shiny, female_reg, female_shiny, right_reg, right_shiny]:
                        if path.exists() and str(path) not in written_paths:
                            out_file.write(str(path) + "\n")
                            written_paths.add(str(path))

print(f"Number of unique Pokémon: {len(unique_pokemon)}")