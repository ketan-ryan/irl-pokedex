import json
import re
import requests
from bs4 import BeautifulSoup
from urllib.parse import urlparse, urljoin

PAGES = [
    "https://pokeshopper.net/rotomplushdex/buildabear",
    "https://pokeshopper.net/rotomplushdex/shiny"
]

HEADERS = {
    "User-Agent": "Mozilla/5.0"
}

IMAGE_EXTENSIONS = (".jpg", ".jpeg", ".png", ".webp")

def load_pokemon_list(path="all_pokemon.json"):
    with open(path, "r", encoding="utf-8") as f:
        return [p.lower() for p in json.load(f)]

def normalize_url(base, url):
    url = url.strip()
    if url.startswith("//"):
        return "https:" + url
    if url.startswith("/"):
        return urljoin(base, url)
    return url

def canonical_image_url(url):
    """
    Remove query params and fragments.
    Keeps only the base image URL.
    """
    parsed = urlparse(url)
    return f"{parsed.scheme}://{parsed.netloc}{parsed.path}"

def extract_image_urls(html, base_url):
    soup = BeautifulSoup(html, "html.parser")
    urls = set()

    # <img src=...>
    for img in soup.find_all("img"):
        if img.get("src"):
            urls.add(normalize_url(base_url, img["src"]))

        # srcset support
        if img.get("srcset"):
            for part in img["srcset"].split(","):
                urls.add(normalize_url(base_url, part.split()[0]))

    # inline styles: background-image:url(...)
    for tag in soup.find_all(style=True):
        matches = re.findall(r"url\((.*?)\)", tag["style"])
        for match in matches:
            urls.add(normalize_url(base_url, match.strip("'\"")))

    # raw links in page
    for link in re.findall(r"https?://[^\s\"']+", html):
        if link.lower().endswith(IMAGE_EXTENSIONS):
            urls.add(link)

    # final filter: image extensions OR impro.usercontent.one
    return {
        u for u in urls
        if u.lower().endswith(IMAGE_EXTENSIONS) or "usercontent.one" in u
    }

def clean_filename(url):
    return urlparse(url).path.split("/")[-1].lower()

def identify_pokemon(filename, pokemon_list):
    for pokemon in pokemon_list:
        if pokemon in filename:
            return pokemon
    return None

def scrape():
    pokemon_list = load_pokemon_list()

    results = {p: [] for p in pokemon_list}
    results["misc"] = []

    all_images = set()

    for page in PAGES:
        resp = requests.get(page, headers=HEADERS, timeout=20)
        resp.raise_for_status()
        # all_images |= extract_image_urls(resp.text, page)
        raw_images = extract_image_urls(resp.text, page)
        all_images |= {canonical_image_url(u) for u in raw_images}

    for url in sorted(all_images):
        filename = clean_filename(url)
        pokemon = identify_pokemon(filename, pokemon_list)

        if pokemon:
            results[pokemon].append(url)
        else:
            results["misc"].append(url)

    # remove empty pokemon entries
    results = {k: v for k, v in results.items() if v or k == "misc"}

    return results

if __name__ == "__main__":
    data = scrape()
    with open("pokemon_plush_images.json", "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)

    print(f"Images found: {sum(len(v) for v in data.values())}")
    print(f"Unidentified (misc): {len(data.get('misc', []))}")
