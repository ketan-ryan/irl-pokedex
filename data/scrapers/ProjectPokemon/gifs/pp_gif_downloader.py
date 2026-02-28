import requests
from bs4 import BeautifulSoup
from pathlib import Path
import time

out_dir = Path('C:/Users/kyure/Desktop/Code projects/irl-pokedex/data/gifs')

for i in range(1, 9):
    suffix = i - 1
    if i == 8:
        suffix = 123
    url = f'https://projectpokemon.org/home/docs/spriteindex_148/3d-models-generation-{i}-pok%C3%A9mon-r9{suffix}/'
    response = requests.get(url)
    pp_html = response.text

    soup = BeautifulSoup(pp_html, 'html.parser')
    table = soup.find('table')

    rows = []
    for tr in table.find_all('tr'):
        for td in tr.find_all('td'):
            name = td.find('img')['alt']
            gif_src = td.find('img')['src']

            # Shinies don't need separate sprites when scrolling the dex
            # TODO future improvement: mega evolution, dynamax dex
            if 'shiny' in gif_src or '-mega' in gif_src: continue

            out_gif = Path(f'{out_dir}/{name}')
            if out_gif.exists(): continue

            gif_response = requests.get(gif_src)
            
            with open(f'{out_dir}/{name}', 'wb') as fp:
                fp.write(gif_response.content)
        time.sleep(0.1)