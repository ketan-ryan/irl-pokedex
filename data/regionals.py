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
                original = key[key.find(' ') + 1:]

                # Link back to the original pokemon
                if 'relations' in data[key] and original not in data[key]['relations']:
                    data[key]['relations'].append(original)
                else:
                    data[key]['relations'] = [original]
                
                # Link the original pokemon to the regional form
                if original in data:
                    if 'relations' in data[original] and key not in data[original]['relations']:
                        data[original]['relations'].append(key)
                    else:
                        data[original]['relations'] = [key]
                    data[original]['base'] = True
                else:
                    print(f"Original pokemon {original} not found in the dex.")

with open(dex_path, 'w', encoding='utf-8') as fp:
    json.dump(data, fp, indent=4, ensure_ascii=False)
