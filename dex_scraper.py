import json
import time

from bs4 import BeautifulSoup
import requests
from tqdm import tqdm


pokedb_url = 'https://pokemondb.net/pokedex/national'
response = requests.get(pokedb_url)
pokedb_html = response.text

soup = BeautifulSoup(pokedb_html, 'html.parser')
pokemon = [a['href'] for a in soup.find_all('a', href=True) if '/pokedex/' in a['href']]
# Pokedex categories come before pokemon
pokemon = pokemon[pokemon.index('/pokedex/bulbasaur'):]

entries = {}
for poke_url in tqdm(pokemon):
    pokemon_name = poke_url[poke_url.rfind('/') + 1:]
    if pokemon_name in entries:
        continue

    data = {}
    url = f'https://pokemondb.net{poke_url}'
    response = requests.get(url)
    poke_soup = BeautifulSoup(response.text, 'html.parser')
    
    # The first h2 is the pokedex data table
    header = poke_soup.h2
    dex_table_data = header.find_next_sibling('table')
    for row in dex_table_data.find_all('tr'):
        th = row.find('th')
        td = row.find('td')
        if th and td:
            key = th.get_text(strip=True).split(' ')[0]
            match key:
                case 'National':
                    data['number'] = td.get_text(strip=True)
                case 'Type':
                    types = td.find_all('a')
                    type_str = types[0].get_text()
                    if len(types) == 2:
                        type_str = type_str + '/' + types[1].get_text()
                    data['type'] = type_str
                case 'Species':
                    data['species'] = td.get_text()
                case 'Height':
                    data['height'] = ''.join(ch for ch in td.get_text() if ch.isprintable()).strip()
                case 'Weight':
                    data['weight'] = ''.join(ch for ch in td.get_text() if ch.isprintable()).strip()
                case 'Abilities':
                    ab_list = []
                    abilities = td.find_all('a')
                    for ability in abilities:
                        ab_list.append(ability.get_text())
                    data['abilities'] = ab_list

    # Get dex entries
    dex_dict = {}
    
    dex_header = poke_soup.find('h2', string='Pokédex entries') 
    if dex_header:
        dex_entries = dex_header.find_next_sibling('div').find('table')
        for row in dex_entries.find_all('tr'):
            th = row.find('th')
            td = row.find('td')
            if th and td:
                game_spans = th.find_all('span')
                games = [game.get_text() for game in game_spans]

                game_str = '/'.join(games)
                dex_dict['game_str'] = td.get_text()
    
    data['dex_entries'] = dex_dict
    
    entries[pokemon_name] = data
    time.sleep(0.1)  # don't hit their servers too fast

with open('pokedex.json', 'w') as fp:
    json.dump(entries, fp, indent=4)