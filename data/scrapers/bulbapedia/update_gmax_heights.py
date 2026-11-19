import argparse
import json
import re
import unicodedata

import requests
from bs4 import BeautifulSoup

URL = 'https://bulbapedia.bulbagarden.net/wiki/Gigantamax'
WEIGHT = '????.? lbs'
GENERIC = {'both forms', 'any form'}  # Toxtricity / Alcremie: one gmax form for all forms

# e.g. 91'10"+ (28.0+ m)   (accepts ' or ′ and " or ″)
HEIGHT_RE = re.compile(r"""(\d+)\s*['′]\s*(\d+)\s*["″]\s*\+\s*\(\s*([\d.]+)\s*\+?\s*m\s*\)""")
DEX_RE = re.compile(r'^#\d{4}$')


def normalize(s: str) -> str:
    s = unicodedata.normalize('NFKD', s).encode('ascii', 'ignore').decode('ascii')
    return ''.join(c for c in s if c.isalpha()).lower()


def scrape_heights() -> dict:
    """Return {normalized pokemon name: height string} from the 'After Gigantamax' column."""
    html = requests.get(URL, headers={'User-Agent': 'pokedex-gmax-updater/1.0'}).text
    soup = BeautifulSoup(html, 'html.parser')

    heights = {}
    last = None  # for rows that share the previous row's height via rowspan (Appletun)
    for row in soup.find_all('tr'):
        texts = [c.get_text(' ', strip=True).replace('\xa0', ' ') for c in row.find_all(['td', 'th'])]
        dex_idx = next((i for i, t in enumerate(texts) if DEX_RE.match(t)), None)
        # rows without a dex cell (Urshifu's 2nd form) start with the name cell
        name_idx = dex_idx + 1 if dex_idx is not None else 0
        if name_idx >= len(texts):
            continue
        name_text = texts[name_idx]
        rest = ' '.join(texts[name_idx + 1:])

        m = HEIGHT_RE.search(rest)
        if m:
            height = f'{m[3]}+ m ({m[1]}′{m[2]}″+)'
            last = height
        elif dex_idx is not None and last and 'G-Max' in rest:
            height = last  # shared gmax form, no height cell of its own
        else:
            continue

        main = re.sub(r'\s*\(.*?\)\s*', ' ', name_text).strip()
        paren = re.search(r'\((.*?)\)', name_text)
        paren = paren[1].strip() if paren else None
        if paren and paren.lower() not in GENERIC:
            heights[normalize(f'{main} {paren}')] = height   # "Urshifu Single Strike Style"
            heights.setdefault(normalize(main), height)      # plain "Urshifu" -> first form
        else:
            heights[normalize(main)] = height
    return heights


def lookup(heights: dict, gmax_key: str):
    name = normalize(gmax_key.removeprefix('Gigantamax ').strip())
    if name in heights:
        return heights[name]
    # fall back to the longest Bulbapedia name that prefixes ours,
    # e.g. "Toxtricity Amped Form" -> "Toxtricity"
    for key in sorted(heights, key=len, reverse=True):
        if name.startswith(key):
            return heights[key]
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('json_path', nargs='?', default='pokedex.json')
    ap.add_argument('--out', help='write here instead of overwriting the input')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    with open(args.json_path, encoding='utf-8') as fp:
        data = json.load(fp)

    heights = scrape_heights()
    print(f'Scraped {len(heights)} gmax heights from Bulbapedia')
    if not heights:
        raise SystemExit('No heights parsed, the page layout may have changed.')

    gmax_keys = {e['gmax'] for e in data.values() if e.get('gmax')}
    updated = 0
    for gmax_key in sorted(gmax_keys):
        if gmax_key not in data:
            print(f'! "{gmax_key}" is referenced by a gmax field but has no entry, skipping')
            continue
        height = lookup(heights, gmax_key)
        if height is None:
            print(f'! no Bulbapedia height found for "{gmax_key}"')
            continue
        entry = data[gmax_key]
        if entry.get('height') != height or entry.get('weight') != WEIGHT:
            print(f'{gmax_key}: {entry.get("height")} -> {height}')
            entry['height'] = height
            entry['weight'] = WEIGHT
            updated += 1

    print(f'Updated {updated} entries')
    if not args.dry_run:
        with open(args.out or args.json_path, 'w', encoding='utf-8') as fp:
            json.dump(data, fp, indent=4, ensure_ascii=False)
            fp.write('\n')


if __name__ == '__main__':
    main()