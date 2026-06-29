import json
from pathlib import Path

dex_path = Path('pokedex/assets/resources/pokedex.json')
regions = ['Kanto', 'Johto', 'Hoenn', 'Sinnoh', 'Unova', 'Kalos', 'Alola', 'Galar', 'Hisui', 'Paldea']
with open(dex_path, 'r', encoding='utf-8') as fp:
    data = json.load(fp)
    for key in data.keys():
        for region in regions:
            if region.lower() in key.lower():
                data[key]['region'] = region

with open(dex_path, 'w', encoding='utf-8') as fp:
    json.dump(data, fp, indent=4, ensure_ascii=False)
