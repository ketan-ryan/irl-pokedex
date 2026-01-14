import json
import time
import random
from pathlib import Path
from pokemontcgsdk import Card, RestClient, PokemonTcgException
from urllib.error import HTTPError

# CONFIG
POKEMON_LIST_PATH = "all_pokemon.json"
OUTPUT_JSON = "pokemon_tcg_metadata.json"
API_KEY_PATH = "secrets.txt"

MAX_RETRIES = 5
SLEEP_BETWEEN_POKEMON = 1

# SETUP
with open(API_KEY_PATH, "r") as fp:
    RestClient.configure(fp.readline().strip())

with open(POKEMON_LIST_PATH, "r", encoding="utf-8") as fp:
    pokemon_names = json.load(fp)

# HELPERS
def safe_card_query(pokemon, max_retries=MAX_RETRIES):
    query_name = pokemon

    # query = (
    #     f'name:"{query_name}" '
    #     f'(rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")'
    # )

    for attempt in range(max_retries):
        try:
            return Card.where(
                page=1,
                pageSize=250,
                q = f'name:{query_name} (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")'
                # q=query
            )
        except (HTTPError, PokemonTcgException) as e:
            wait = (2 ** attempt) + random.uniform(0, 1.5)
            print(f"[WARN] {pokemon} failed, retrying in {wait:.2f}s")
            time.sleep(wait)

    print(f"[FAIL] Giving up on {pokemon}")
    return []


def scrape_card_metadata(pokemon):
    cards = safe_card_query(pokemon)
    if not cards:
        return {}

    results = {}
    for card in cards:
        key = f"{card.name}-{card.id}"
        results[key] = card.images.small

    return results

# MAIN
if Path(OUTPUT_JSON).exists():
    with open(OUTPUT_JSON, "r", encoding="utf-8") as fp:
        all_metadata = json.load(fp)
else:
    print("no metadata exists...creating new")
    all_metadata = {}

if Path("onlyone.json").exists():
    with open("onlyone.json", "r", encoding="utf-8") as fp:
        onlyone = json.load(fp)
if Path("onlytwo.json").exists():
    with open("onlytwo.json", "r", encoding="utf-8") as fp:
        onlytwo = json.load(fp)
if Path("onlythree.json").exists():
    with open("onlythree.json", "r", encoding="utf-8") as fp:
        onlythree = json.load(fp)
if Path("onlyfour.json").exists():
    with open("onlyfour.json", "r", encoding="utf-8") as fp:
        onlyfour = json.load(fp)

for pokemon in pokemon_names:
    if pokemon in all_metadata and all_metadata[pokemon]:
        continue
    # if len(all_metadata[pokemon]) >= 5:
    #     continue
    # elif pokemon == "nidoran♀" or pokemon == "nidoran♂": # try commenting out and see if it works without this fix
    #     pokemon = "nidoran"
    elif Path("onlyone.json").exists() and pokemon in onlyone and len(all_metadata[pokemon]) == 1:
        continue
    elif Path("onlytwo.json").exists() and pokemon in onlytwo and len(all_metadata[pokemon]) == 2:
        continue
    elif Path("onlythree.json").exists() and pokemon in onlythree and len(all_metadata[pokemon]) == 3:
        continue
    elif Path("onlyfour.json").exists() and pokemon in onlyfour and len(all_metadata[pokemon]) == 4:
        continue
    # elif ' ' in pokemon:
    #     continue

    print(f"Scraping {pokemon}...")
    data = scrape_card_metadata(pokemon)

    if not data:
        print(f"[SKIP] No new data for {pokemon}")
        continue

    existing = all_metadata.get(pokemon, {})

    # Merge: new cards override duplicates, old cards preserved
    merged = {**existing, **data}

    if len(merged) > len(existing):
        print(f"[ADD] {pokemon}: +{len(merged) - len(existing)} new cards")
    else:
        print(f"[SKIP] No new data for {pokemon}")

    all_metadata[pokemon] = merged

    with open(OUTPUT_JSON, "w", encoding="utf-8") as fp:
        json.dump(all_metadata, fp, indent=2, sort_keys=True)

    time.sleep(SLEEP_BETWEEN_POKEMON)

print(f"\nSaved metadata for {len(all_metadata)} Pokémon → {OUTPUT_JSON}")
