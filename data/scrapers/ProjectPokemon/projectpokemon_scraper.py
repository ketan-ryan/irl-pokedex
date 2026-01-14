import json
import re
import requests
from bs4 import BeautifulSoup
from tqdm import tqdm

GEN_URLS = [
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-1-r128/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-2-r129/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-3-r130/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-4-r131/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-5-r132/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-6-r133/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-7-r134/",
    "https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-8-r135/",
]

# Load existing JSON if it exists
try:
    with open("pokemon_home_images_full.json") as f:
        entries = json.load(f)
except FileNotFoundError:
    entries = {}

FILENAME_REGEX = re.compile(
    r"poke_capture_(\d{4})_(\d{3})_([a-z]{2})_(n|g)_\d+_f_(n|r)\.png"
)

def ensure(dex, form, gender):
    entries.setdefault(dex, {})
    entries[dex].setdefault(form, {})
    entries[dex][form].setdefault(gender, {
        "normal": None,
        "shiny": None
    })

for url in tqdm(GEN_URLS, desc="Scraping HOME sprites"):
    html = requests.get(url, timeout=30).text
    soup = BeautifulSoup(html, "html.parser")

    for img in soup.find_all("img", src=True):
        src = img["src"]

        if "poke_capture_" not in src:
            continue

        match = FILENAME_REGEX.search(src)
        if not match:
            continue

        dex = int(match.group(1))
        form_index = match.group(2)  # '000', '001', etc.
        gender_tag = match.group(3)
        form_tag = match.group(4)
        color_tag = match.group(5)

        # Determine base / mega / alternate form
        if form_tag == "g":
            form = "gigantamax"
        else:
            form = "base" if form_index == "000" else f"alt_{form_index}"

        gender = (
            "default" if gender_tag in ["mf", "uk", "mo", "fo"]
            else "male" if gender_tag == "md"
            else "female"
        )

        color = "shiny" if color_tag == "r" else "normal"

        ensure(str(dex), form, gender)  # make sure dex is always a string
        entries[str(dex)][form][gender][color] = src

with open("pokemon_home_images_full.json", "w") as f:
    json.dump(dict(sorted(entries.items(), key=lambda x: int(x[0]))), f, indent=4)

print(f"Total Pokémon scraped: {len(entries)}")