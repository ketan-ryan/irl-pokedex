import json
import sys
import time
import unicodedata

import requests
from bs4 import BeautifulSoup
from tqdm import tqdm

IN_PATH = sys.argv[1] if len(sys.argv) > 1 else 'pokedex.json'
OUT_PATH = sys.argv[2] if len(sys.argv) > 2 else IN_PATH  # overwrite unless told otherwise
SITE = 'https://pokemondb.net'


def normalize(s: str) -> str:
    """'Flabébé' -> 'flabebe', "Farfetch'd" -> 'farfetchd', 'Mr. Mime' -> 'mrmime'"""
    s = unicodedata.normalize('NFKD', s).encode('ascii', 'ignore').decode('ascii')
    return ''.join(c for c in s if c.isalpha()).lower()


def parse_dex_table(table) -> dict:
    """Same row parsing as the main scraper: 'Sword' -> text, 'Red/Blue' -> text."""
    entries = {}
    for row in table.find_all('tr'):
        th, td = row.find('th'), row.find('td')
        if th and td:
            games = [s.get_text() for s in th.find_all('span')] or [th.get_text(strip=True)]
            entries['/'.join(games)] = td.get_text()
    return entries


def gmax_tables(poke_soup):
    """Yield (heading_text, table) for each 'Gigantamax...' h3 in the Pokédex entries section."""
    header = poke_soup.find('h2', string='Pokédex entries')
    if header is None:
        return
    for sib in header.find_next_siblings():
        if sib.name == 'h2':  # reached the next section
            break
        if sib.name == 'h3' and sib.get_text(strip=True).startswith('Gigantamax'):
            div = sib.find_next_sibling('div')
            table = div.find('table') if div else None
            if table:
                yield sib.get_text(strip=True), table


with open(IN_PATH, encoding='utf-8') as fp:
    data = json.load(fp)

# base key -> gmax key, for gmax entries that exist but have no dex entries yet
todo = {}
for key, entry in data.items():
    gmax_key = entry.get('gmax')
    if not gmax_key:
        continue
    if gmax_key not in data:
        print(f'! {key}: "{gmax_key}" has no entry in the json, skipping')
    elif not data[gmax_key].get('dex_entries'):
        todo[key] = gmax_key

print(f'{len(todo)} gmax entries need dex entries')

# map normalized name -> pokemondb slug, using the national dex list
soup = BeautifulSoup(requests.get(f'{SITE}/pokedex/national').content, 'html.parser', from_encoding='utf-8')
hrefs = [a['href'] for a in soup.find_all('a', href=True) if '/pokedex/' in a['href']]
hrefs = hrefs[hrefs.index('/pokedex/bulbasaur'):]
slugs = {normalize(h.rsplit('/', 1)[-1]): h.rsplit('/', 1)[-1] for h in hrefs}

updated = 0
for base_key, gmax_key in tqdm(todo.items()):
    slug = slugs.get(normalize(base_key))
    if slug is None:
        tqdm.write(f'! no pokemondb page found for "{base_key}"')
        continue

    page = requests.get(f'{SITE}/pokedex/{slug}')
    poke_soup = BeautifulSoup(page.text, 'html.parser')

    found = list(gmax_tables(poke_soup))
    if not found:
        tqdm.write(f'! no Gigantamax section on {slug}')
    for heading, table in found:
        # plain "Gigantamax" heading = the entry we're looking for; anything else
        # (e.g. a style-specific heading) only applies if it matches a key exactly
        target = gmax_key if heading == 'Gigantamax' else heading
        if target not in data:
            tqdm.write(f'! {slug}: heading "{heading}" has no matching key, skipped')
            continue
        data[target]['dex_entries'] = parse_dex_table(table)
        updated += 1
    time.sleep(0.1)  # don't hit their servers too fast

with open(OUT_PATH, 'w', encoding='utf-8') as fp:
    json.dump(data, fp, indent=4, ensure_ascii=False)
    fp.write('\n')

print(f'Updated {updated} gmax entries -> {OUT_PATH}')