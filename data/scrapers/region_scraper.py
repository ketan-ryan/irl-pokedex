import json
import requests
from bs4 import BeautifulSoup
from pathlib import Path

url = "https://rankedboost.com/pokemon/pokedex/?gen="
dex_path = Path('pokedex/assets/resources/pokedex.json')
regions = ['Kanto', 'Johto', 'Hoenn', 'Sinnoh', 'Unova', 'Kalos', 'Alola', 'Galar', 'Paldea']
with open(dex_path, 'r', encoding='utf-8') as fp:
    data = json.load(fp)

for i in range(1, 10):
    query_url = "https://rankedboost.com/pokemon/pokedex/?gen=" + str(i)

    response = requests.get(query_url)
    soup = BeautifulSoup(response.text, "html.parser")

    for mon in soup.find("div", class_="pdex-results").children:
        name = mon["data-name"]
        cap_name = name.title()
        if cap_name not in data:
            print(f'Skipping pokemon {cap_name}')
            continue

        dex_data = data[cap_name]
        if 'region' not in dex_data:
            dex_data['region'] = regions[i - 1]

with open(dex_path, 'w', encoding='utf-8') as fp:
    json.dump(data, fp, indent=4, ensure_ascii=False)
