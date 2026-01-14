import json
import requests
from bs4 import BeautifulSoup

url = "https://pokemondb.net/pokedex/national"
response = requests.get(url)
soup = BeautifulSoup(response.text, "html.parser")

entries = {}

for card in soup.find_all("div", class_="infocard"):
    name_tag = card.find("a", class_="ent-name")
    img_tag = card.find("img", class_="img-sprite")

    if not name_tag or not img_tag:
        continue

    name = name_tag.get_text(strip=True).lower()
    image_url = img_tag["src"]

    entries[name] = {
        "image_url": image_url
    }

with open("pokemon_images.json", "w") as f:
    json.dump(entries, f, indent=4)
