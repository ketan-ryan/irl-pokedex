import json
from collections import defaultdict
import re
from difflib import get_close_matches

INPUT_JSON = "rotom_plushdex_plush_only.json"
ALL_POKEMON_JSON = "all_pokemon.json"
OUTPUT_JSON = "rotom_plushdex_plush_cleaned.json"

REGIONAL_PREFIXES = ["alolan", "galarian", "hisuian", "paldean"]

# Map HQ filenames to Nidoran forms
NIDORAN_FEMALE_PATTERNS = re.compile(r"nidoran(h|f)?hq", re.IGNORECASE)
NIDORAN_MALE_PATTERNS = re.compile(r"nidoranm(h)?hq", re.IGNORECASE)

# Nidoran dex numbers
NIDORAN_F_DEX = {29}
NIDORAN_M_DEX = {32}

SUFFIXES = ["hq", "base1", "base2", "base3"]

# Load all valid Pokémon names
with open(ALL_POKEMON_JSON, "r", encoding="utf-8") as f:
    ALL_POKEMON = set(json.load(f))
ALL_POKEMON_LOWER = {p.lower(): p for p in ALL_POKEMON}  # map lower->proper

def map_pokemon_key(key: str, url: str = None) -> str:
    key_lower = key.lower()

    # Nidoran special case
    if "nidoran" in key_lower and url:
        if "plushdata-029-" in url:
            return "nidoran♀"
        if "plushdata-032-" in url:
            return "nidoran♂"
        fname = url.split("/")[-1].split("?")[0].lower()
        if NIDORAN_FEMALE_PATTERNS.search(fname):
            return "nidoran♀"
        if NIDORAN_MALE_PATTERNS.search(fname):
            return "nidoran♂"
        if "m" in key_lower:
            return "nidoran♂"
        return "nidoran♀"

    # Regional variants
    for prefix in REGIONAL_PREFIXES:
        if key_lower.startswith(prefix):
            key_lower = key_lower[len(prefix):]
            key_lower = re.sub(r"^[^a-z0-9]+", "", key_lower)
            break

    # Strip known suffixes
    for suffix in ["hq", "base1", "base2", "base3"]:
        if key_lower.endswith(suffix):
            key_lower = key_lower[:-len(suffix)]
            break

    # Correct misspellings
    if key_lower in ALL_POKEMON_LOWER:
        return ALL_POKEMON_LOWER[key_lower]
    else:
        matches = get_close_matches(key_lower, ALL_POKEMON_LOWER.keys(), n=1, cutoff=0.7)
        if matches:
            return ALL_POKEMON_LOWER[matches[0]]
        else:
            return key


# Load JSON
with open(INPUT_JSON, "r", encoding="utf-8") as f:
    data = json.load(f)

merged_data = defaultdict(set)

# Merge URLs under mapped keys
for pokemon, urls in data.items():
    for url in urls:
        mapped_key = map_pokemon_key(pokemon, url)
        merged_data[mapped_key].add(url)

# Convert sets to sorted lists
final_data = {k: sorted(v) for k, v in merged_data.items()}

# Save cleaned JSON
with open(OUTPUT_JSON, "w", encoding="utf-8") as f:
    json.dump(final_data, f, indent=2, ensure_ascii=False)

print(f"Saved cleaned plushdex with {len(final_data)} Pokémon → {OUTPUT_JSON}")
