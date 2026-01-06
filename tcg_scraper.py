from io import BytesIO
import json
from pathlib import Path
import requests
from typing import List
import time, random
from concurrent.futures import ThreadPoolExecutor, as_completed
from urllib.error import HTTPError

from PIL import Image
from pokemontcgsdk import Card, RestClient, PokemonTcgException
from tqdm import tqdm

# GOOGLE COLAB IMPORTS
# !pip install pokemontcgsdk
# from google.colab import drive
# from google.colab import files
# drive.mount('/content/drive')


# BASE DIRECTORY WHERE TCG IMAGES LIVE
BASE_DIR = Path(r"G:\My Drive\IRL Pokedex\TCG imgs") # FOR GOOGLE DRIVE DESKTOP
# BASE_DIR = Path("/content/drive/My Drive/IRL Pokedex/TCG imgs") # FOR GOOGLE COLAB

FAILED_PATH = Path(__file__).resolve().parent / "failed_pokemon.json" # LOCAL
# FAILED_PATH = BASE_DIR / "failed_pokemon.json" # CLOUD (FOR GOOGLE COLAB)

if FAILED_PATH.exists():
    with open(FAILED_PATH, "r") as fp:
        failed_pokemon = set(json.load(fp))
else:
    failed_pokemon = set()

names = []
with open('classes_cleaned.json', 'r') as fp:
    names = json.load(fp)

with open('secrets.txt', 'r') as fp:
    # Create an account and get a key from https://dev.pokemontcg.io/
    # Save it to a file called secrets.txt
    key = fp.readline().strip()
    RestClient.configure(key)


# HAS PAGE LOOP
# def safe_card_query(pokemon, max_retries=5, max_pages=5):
#     # query_name = pokemon
#     query_name = pokemon.replace("-", " ")
#     for attempt in range(max_retries):
#         results = []
#         try:
#             for page in range(1, max_pages + 1):
#                 batch = Card.where(
#                     page=page,
#                     pageSize=250,
#                     q=f'name:"{query_name}"')# (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")')
#                 if not batch:
#                     break
#                 results.extend(batch)
#                 time.sleep(0.4)

#             return results
        
#         except HTTPError as e:
#             wait = (2 ** attempt) + random.uniform(0, 1.5)
#             print(f"HTTPError for {query_name}, retrying in {wait:.2f}s:")
#             print("  code:", e.code)
#             print("  reason:", e.reason)
#             time.sleep(wait)

#         except PokemonTcgException as e:
#             wait = (2 ** attempt) + random.uniform(0, 1.5)
#             print(f"PokemonTcgException for {query_name}, retrying in {wait:.2f}s:")
#             print("    attrs:", vars(e))
#             time.sleep(wait)

#     print(f"Giving up on {query_name}")
#     return []

def safe_card_query(pokemon, max_retries=5):
    # query_name = pokemon
    query_name = pokemon.replace("-", " ")
    for attempt in range(max_retries):
        try:
            return Card.where(
                page=1,
                pageSize=250,
                q=f'name:"{query_name}"')# (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")')
                # q = f'name:"{pokemon}" (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")')
                
                # q=f'!name:{pokemon} (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")')
            
        except HTTPError as e:
            wait = (2 ** attempt) + random.uniform(0, 1.5)
            print(f"HTTPError for {query_name}, retrying in {wait:.2f}s:")
            print("  code:", e.code)
            print("  reason:", e.reason)
            time.sleep(wait)

        except PokemonTcgException as e:
            wait = (2 ** attempt) + random.uniform(0, 1.5)
            print(f"PokemonTcgException for {query_name}, retrying in {wait:.2f}s:")
            print("    attrs:", vars(e))
            time.sleep(wait)

    print(f"Giving up on {query_name}")
    return []

def download_and_crop(card, pokemon, base_dir):
    cname = card.name
    cid = card.id
    curl = card.images.small

    try:
        response = requests.get(curl, timeout=10)
        response.raise_for_status()

        img = Image.open(BytesIO(response.content))
        cropped = img.crop((26, 35 + 5, 219 - 5, 160))

        path = base_dir / "crops" / pokemon / f"{cname}-{cid}.png"
        if path.exists():
            return f"{cname}-{cid}", curl
        cropped.save(path)

        return f"{cname}-{cid}", curl

    except Exception as e:
        print(f"Image failed {pokemon} {cname}-{cid}: {e}")
        return None


pbar = tqdm(names)
# pbar = tqdm(reversed(names))
for pokemon in pbar:
    pokemon_dir = BASE_DIR / "crops" / pokemon
    pokemon_dir.mkdir(parents=True, exist_ok=True)
    json_path = pokemon_dir / f"{pokemon}.json"

    if pokemon not in failed_pokemon:
        continue

    # if json_path.exists():
    #     try:
    #         with open(json_path, "r") as fp:
    #             data = json.load(fp)

    #         entries = data.get(pokemon, {})

    #         if len(entries) >= 1: # or pokemon in failed_pokemon:
    #             print(f"Skipping {pokemon} (already done)")
    #             continue
    #         else:
    #             print(f"Retrying {pokemon} (empty JSON)")
    #             failed_pokemon.add(pokemon)

    #     except json.JSONDecodeError:
    #         print(f"Corrupt JSON for {pokemon}, retrying")
    #         failed_pokemon.add(pokemon)
    # else:
    #     print(f"WARNING: missing JSON for {pokemon}")

    pbar.set_description(pokemon)
    cards = safe_card_query(pokemon)

    if not cards:
        print(f"No cards returned for {pokemon}")
        failed_pokemon.add(pokemon)

        with open(FAILED_PATH, "w") as fp:
            json.dump(sorted(failed_pokemon), fp, indent=2, sort_keys=True)

        if not json_path.exists():
            with open(json_path, "w") as fp:
                json.dump({pokemon: {}}, fp, indent=2, sort_keys=True)

        time.sleep(1.0) # + random.uniform(0, 0.75))
        continue

    unique_cards = {}
    for card in cards:
        unique_cards[card.id] = card

    cards = list(unique_cards.values())

    # Load existing data if present
    if json_path.exists():
        try:
            with open(json_path, "r") as fp:
                paths = json.load(fp)
        except json.JSONDecodeError:
            paths = {pokemon: {}}
    else:
        paths = {pokemon: {}}

    existing_keys = set(paths.get(pokemon, {}).keys())

    # Filter cards we still need
    cards_to_download = []
    for card in cards:
        # print (card.name, card.id, card.images.small)
        key = f"{card.name}-{card.id}"
        img_path = BASE_DIR / "crops" / pokemon / f"{key}.png"

        if key not in existing_keys or not img_path.exists():
            cards_to_download.append(card)

    if not cards_to_download:
        print(f"No new cards for {pokemon}")
    else:
        # W/O MULTITHREADING
        for card in cards_to_download:
            result = download_and_crop(card, pokemon, BASE_DIR)
            if result is None:
                continue

            key, url = result
            paths[pokemon][key] = url
        
        #W/ MULTITHREADING
        # with ThreadPoolExecutor(max_workers=4) as executor:
        #     futures = [
        #         executor.submit(download_and_crop, card, pokemon, BASE_DIR)
        #         for card in cards_to_download
        #     ]

        #     for future in as_completed(futures):
        #         result = future.result()
        #         if result is None:
        #             continue

        #         key, url = result
        #         paths[pokemon][key] = url

    # Write (append-safe)
    with open(json_path, "w") as fp:
        json.dump(paths, fp, indent=2, sort_keys=True)

    # Update failed list
    has_images = any(
        (BASE_DIR / "crops" / pokemon).glob("*.png")
    )

    if not paths[pokemon] and not has_images:
        failed_pokemon.add(pokemon)
    else:
        failed_pokemon.discard(pokemon)

    with open(FAILED_PATH, "w") as fp:
        json.dump(sorted(failed_pokemon), fp, indent=2, sort_keys=True)

    time.sleep(1.0) # + random.uniform(0, 0.75))