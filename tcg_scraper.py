from typing import List
from pokemontcgsdk import Card, RestClient
from PIL import Image
import requests
from io import BytesIO
import json
from pathlib import Path
from tqdm import tqdm


names = []
with open('classes.json', 'r') as fp:
    names = json.load(fp)

with open('secrets.txt', 'r') as fp:
    # Create an account and get a key from https://dev.pokemontcg.io/
    # Save it to a file called secrets.txt
    key = fp.readline().strip()
    RestClient.configure(key)


pbar = tqdm(names)
for pokemon in pbar:
    pbar.set_description(pokemon)
    cards: List[Card] = Card.where(page=1, pageSize=250, q=f'!name:{pokemon} (rarity:Common OR rarity:Uncommon OR rarity:Rare OR rarity:"Rare Shiny")')

    paths = {}
    paths[pokemon] = {}
    for card in cards:
        cname = card.name
        cid = card.id
        curl = card.images.small

        # print (card.name, card.id, card.images.small)
        response = requests.get(curl)
        image_data = BytesIO(response.content)
        img = Image.open(image_data)
        cropped = img.crop((26, 35 + 5, 219 - 5, 160))
        
        path = Path(f'crops/{pokemon}/{cname}-{cid}.png')
        path.parent.mkdir(parents=True, exist_ok=True)
        cropped.save(path)

        paths[pokemon][f'{cname}-{cid}'] = curl
    
    with open(f'crops/{pokemon}/{pokemon}.json', 'w+') as fp:
        json.dump(paths, fp)