import json
import time
import unicodedata

from bs4 import BeautifulSoup
import requests
from tqdm import tqdm


pokedb_url = 'https://pokemondb.net/pokedex/national'
response = requests.get(pokedb_url)
pokedb_html = response.text

soup = BeautifulSoup(response.content, 'html.parser', from_encoding='utf-8')
pokemon = [a['href'] for a in soup.find_all('a', href=True) if '/pokedex/' in a['href']]
# Pokedex categories come before pokemon
pokemon = pokemon[pokemon.index('/pokedex/bulbasaur'):]

entries = {}
for poke_url in tqdm(pokemon):
    pokemon_name = poke_url[poke_url.rfind('/') + 1:]
    if pokemon_name in entries:
        continue

    url = f'https://pokemondb.net{poke_url}'
    response = requests.get(url)
    poke_soup = BeautifulSoup(response.text, 'html.parser')
    
    headers = poke_soup.find_all('h2', string="Pokédex data")
    forms = [a.get_text() for a in poke_soup.find('div', class_='sv-tabs-tab-list').find_all('a')]
    try:
        dex_headers = [d for d in poke_soup.find('h2', string='Pokédex entries').find_next_siblings() 
                       if d.name == 'h3' and d.get_text() in forms]
    except AttributeError:
        dex_headers = ['NotFound']

    # No forms - just put all dex entries here
    if len(dex_headers) == 0:
        dex_headers.append(poke_soup.find('h2', string='Pokédex entries'))

    datums = zip(forms, headers, dex_headers)

    # If there are multiple "Pokedex data" headers, this pokemon has regional forms
    for form, header, dex_header in datums:
        dex_table_data = header.find_next_sibling('table')
        data = {}
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
                        data['height'] = td.get_text()
                    case 'Weight':
                        data['weight'] = td.get_text()
                    case 'Abilities':
                        ab_list = []
                        abilities = td.find_all('a')
                        for ability in abilities:
                            ab_list.append(ability.get_text())
                        data['abilities'] = ab_list

        # Get dex entries
        dex_dict = {}
        
        if dex_header != 'NotFound':
            dex_entries = dex_header.find_next_sibling('div').find('table')
            for row in dex_entries.find_all('tr'):
                th = row.find('th')
                td = row.find('td')
                if th and td:
                    game_spans = th.find_all('span')
                    games = [game.get_text() for game in game_spans]

                    game_str = '/'.join(games)
                    dex_dict[f'{game_str}'] = td.get_text()
        else:
            dex_dict = {"Unknown": "This Pokémon has no Pokédex entries. "}

        data['dex_entries'] = dex_dict
        
        # Some forms, like regionals, show up as ex "Alolan Rattata"
        # Others, like giratina, just have ex "Origin Forme" so we prepend the name

        # decomposes accented characters into their base letter + accent mark,
        # reencoding strips accent marks. Ex: Flabébé -> Flabebe
        form_sanitized = unicodedata.normalize('NFKD', form).encode('ascii', 'ignore').decode('ascii')
        form_sanitized = "".join(char for char in form_sanitized if char.isalpha())
        name_sanitized = "".join(char for char in pokemon_name if char.isalpha())

        name_key = form if name_sanitized.lower() in form_sanitized.lower() else f'{pokemon_name} {form}'

        # built-in title method capitalizes the D in farfetch'd, so split only on spaces
        title = ' '.join(word.capitalize() for word in name_key.split(' '))
        entries[title] = data
    time.sleep(0.1)  # don't hit their servers too fast

with open('pokedex_forms_unicode.json', 'w', encoding='utf-8') as fp:
    json.dump(entries, fp, indent=4, ensure_ascii=False)