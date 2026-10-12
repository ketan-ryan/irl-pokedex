import json

with open('pokedex.json', 'r', encoding='utf-8') as fp:
    dex = json.load(fp)
    keys = list(dex.keys())
    with open('classes.json', 'w', encoding='utf-8') as out:
        json.dump(keys, out, indent=4)