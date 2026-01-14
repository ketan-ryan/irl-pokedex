import requests
from bs4 import BeautifulSoup
import json
import time
import re
from collections import defaultdict
from urllib.parse import urlparse, parse_qs, urlunparse

# REGEX
# Capture all known plush formats:
# - Named HQ: pokeshopper-{name}hq.jpg
# - Plushdata or plushdex: pokeshopper-plushdata-{num1}(-{num2})?.jpg
# - Pokeshopper-{pokemon}-plush-{variant}.jpg
# - Pokeshopper-{pokemon}-{variant}.jpg
PLUSH_FILE_RE = re.compile(
    r"pokeshopper-(?:(?P<name>[a-z0-9\-]+)hq|"                     # named HQ
    r"plush(?:data|dex)-(?P<num1>\d+)(?:-(?P<num2>\d+))?|"         # plushdata/plushdex
    r"(?P<nameplush>[a-z0-9\-]+)plush-(?P<var1>\d+)|"              # pokemonplush-{variant}
    r"(?P<namevar>[a-z0-9\-]+)-(?P<var2>\d+))"                     # pokemon-{variant}
    r"\.jpg",
    re.IGNORECASE
)

def canonical_plush_url(url: str) -> str | None:
    """
    Return a clean image URL: keep only scheme + netloc + path.
    Removes query parameters and fragments.
    Only accepts JPG images that are Pokeshopper plushes.
    """
    parsed = urlparse(url)
    if not parsed.path.lower().endswith(".jpg"):
        return None

    if "pokeshopper-" not in parsed.path.lower():
        return None

    # Return only base URL without query string
    return f"{parsed.scheme}://{parsed.netloc}{parsed.path}"


# CONFIG
URLS = [
    "https://pokeshopper.net/rotomplushdex/kanto",
    "https://pokeshopper.net/rotomplushdex/kantodatabase",
    "https://pokeshopper.net/rotomplushdex/johto",
    "https://pokeshopper.net/rotomplushdex/hoenn",
    "https://pokeshopper.net/rotomplushdex/sinnoh",
    "https://pokeshopper.net/rotomplushdex/unova",
    "https://pokeshopper.net/rotomplushdex/kalos",
    "https://pokeshopper.net/rotomplushdex/alola",
    "https://pokeshopper.net/rotomplushdex/galar",
    "https://pokeshopper.net/rotomplushdex/paldea",
]


POKEMON_JSON = "all_pokemon.json"
OUTPUT_JSON = "rotom_plushdex_plush_only.json"

HEADERS = {"User-Agent": "Mozilla/5.0 (PlushDexScraper/3.0)"}

# HELPERS
def load_pokemon_names(path):
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, list):
        names = data
    elif isinstance(data, dict):
        names = data.get("pokemon", data.keys())
    else:
        raise ValueError("Unsupported Pokémon JSON format")
    return {i + 1: name for i, name in enumerate(names)}  # 1-indexed Pokedex

def extract_image_urls(img):
    urls = set()
    for attr in ("data-src", "src"):
        if img.get(attr):
            urls.add(img[attr])
    for attr in ("data-srcset", "srcset"):
        if img.get(attr):
            for part in img[attr].split(","):
                urls.add(part.strip().split(" ")[0])
    return urls

def get_pokemon_from_url(url, pokedex_lookup, last_pokemon=None):
    filename = urlparse(url).path.split("/")[-1]
    m = PLUSH_FILE_RE.search(filename)
    if not m:
        return None

    # Named HQ plush
    if m.group("name"):
        return m.group("name").replace("-", "").lower()

    # Two-number plushdata/plushdex
    num1 = m.group("num1")
    num2 = m.group("num2")
    if num1 and num2:
        return pokedex_lookup.get(int(num1))
    elif num1:
        # Single-number plushdata/plushdex → extra plush, assign last Pokémon
        return last_pokemon

    # pokemonplush-{variant}
    if m.group("nameplush"):
        return m.group("nameplush").replace("-", "").lower()

    # pokemon-{variant}
    if m.group("namevar"):
        return m.group("namevar").replace("-", "").lower()

    return None

# SCRAPER
def scrape():
    pokedex_lookup = load_pokemon_names(POKEMON_JSON)
    results = defaultdict(set)

    for url in URLS:
        print(f"Scraping {url}")
        resp = requests.get(url, headers=HEADERS, timeout=20)
        resp.raise_for_status()
        soup = BeautifulSoup(resp.text, "html.parser")

        last_pokemon = None
        for img in soup.find_all("img"):
            for img_url in extract_image_urls(img):
                if "rotomplushdex" not in img_url:
                    continue

                pokemon = get_pokemon_from_url(img_url, pokedex_lookup, last_pokemon)
                if not pokemon:
                    continue

                last_pokemon = pokemon
                clean = canonical_plush_url(img_url)
                if clean:
                    results[pokemon].add(clean)

        time.sleep(1)

    return {k: sorted(v) for k, v in results.items()}

# MAIN
if __name__ == "__main__":
    data = scrape()
    with open(OUTPUT_JSON, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    print(f"Saved {len(data)} Pokémon plush entries → {OUTPUT_JSON}")
