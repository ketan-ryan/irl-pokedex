import argparse
import json
import random
import re
import time
import unicodedata
from collections import defaultdict
from pathlib import Path
from urllib import robotparser

import requests
from bs4 import BeautifulSoup
from tqdm import tqdm

SITE = 'https://pokemondb.net'
# identify yourself honestly; put a real contact in here
USER_AGENT = 'pokedex-personal-project/1.0 (hobby scraper for use in: https://github.com/ketan-ryan/irl-pokedex)'
DERIVED_PREFIXES = ('Mega ', 'Primal ', 'Gigantamax ')  # inherit gender from their base form


# ---------------------------------------------------------------- polite fetching
class Fetcher:
    """Sequential, rate-limited, cached, robots.txt-aware GETs with backoff."""

    def __init__(self, cache_dir: str, delay: float, use_cache: bool):
        self.session = requests.Session()
        self.session.headers['User-Agent'] = USER_AGENT
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.delay = delay
        self.use_cache = use_cache
        self.last_request = 0.0
        self.robots = robotparser.RobotFileParser()
        try:
            resp = self.session.get(f'{SITE}/robots.txt', timeout=20)
            self.robots.parse(resp.text.splitlines() if resp.ok else [])
            crawl_delay = self.robots.crawl_delay(USER_AGENT)
            if crawl_delay:
                self.delay = max(self.delay, float(crawl_delay))
        except requests.RequestException:
            print('! could not read robots.txt, continuing cautiously')

    def get(self, path: str, cache_name: str) -> str:
        cache_file = self.cache_dir / f'{cache_name}.html'
        if self.use_cache and cache_file.exists():
            return cache_file.read_text(encoding='utf-8')

        url = f'{SITE}{path}'
        if not self.robots.can_fetch(USER_AGENT, url):
            raise PermissionError(f'robots.txt disallows {url}')

        for attempt in range(4):
            # space requests out, with jitter so we aren't a metronome
            wait = self.delay + random.uniform(0, self.delay / 2) - (time.monotonic() - self.last_request)
            if wait > 0:
                time.sleep(wait)
            resp = self.session.get(url, timeout=20)
            self.last_request = time.monotonic()
            resp.encoding = 'utf-8'

            if resp.status_code == 200:
                cache_file.write_text(resp.text, encoding='utf-8')
                return resp.text
            if resp.status_code in (429, 500, 502, 503, 504):
                retry_after = resp.headers.get('Retry-After', '')
                backoff = int(retry_after) if retry_after.isdigit() else 30 * 2 ** attempt
                tqdm.write(f'  HTTP {resp.status_code} from {url}, backing off {backoff}s')
                time.sleep(backoff)
                continue
            resp.raise_for_status()
        raise RuntimeError(f'gave up on {url}')


# ---------------------------------------------------------------- parsing
def normalize(s: str) -> str:
    """'Flabébé' -> 'flabebe', 'Mr. Mime' -> 'mrmime'"""
    s = unicodedata.normalize('NFKD', s).encode('ascii', 'ignore').decode('ascii')
    return ''.join(c for c in s if c.isalpha()).lower()


def form_key(slug: str, form: str) -> str:
    """Same key logic as the main scraper, so keys line up with pokedex.json."""
    name_key = form if normalize(slug) in normalize(form) else f'{slug} {form}'
    return ' '.join(w.capitalize() for w in name_key.split(' '))


def is_derived(key: str) -> bool:
    return key.startswith(DERIVED_PREFIXES)


def gender_letter(text: str):
    t = text.lower()
    if 'genderless' in t:
        return 'N'
    male = re.search(r'([\d.]+)%\s*male', t)
    female = re.search(r'([\d.]+)%\s*female', t)
    m = float(male[1]) if male else 0.0
    f = float(female[1]) if female else 0.0
    if m == 0 and f == 0:
        return None
    if f == 0:
        return 'M'
    if m == 0:
        return 'F'
    return 'B'


def parse_gender(table):
    for row in table.find_all('tr'):
        th, td = row.find('th'), row.find('td')
        if th and td and th.get_text(strip=True) == 'Gender':
            return gender_letter(td.get_text(' ', strip=True))
    return None


def gender_by_form(poke_soup):
    """Yield (form tab name, gender letter or None) for every form tab on the page."""
    tabset = poke_soup.find('div', class_='tabset-basics') or poke_soup
    tab_list = tabset.find('div', class_='sv-tabs-tab-list')
    if tab_list is None:
        return
    for a in tab_list.find_all('a'):
        panel = poke_soup.find(id=a.get('href', '').lstrip('#'))
        h2 = panel.find('h2', string='Breeding') if panel else None
        table = h2.find_next_sibling('table') if h2 else None
        yield a.get_text(strip=True), (parse_gender(table) if table else None)


def national_slugs(fetcher: Fetcher) -> list:
    """Ordered, de-duplicated slugs; index + 1 == national dex number."""
    html = fetcher.get('/pokedex/national', 'national')
    soup = BeautifulSoup(html, 'html.parser')
    hrefs = [a['href'] for a in soup.find_all('a', href=True) if '/pokedex/' in a['href']]
    hrefs = hrefs[hrefs.index('/pokedex/bulbasaur'):]
    slugs, seen = [], set()
    for h in hrefs:
        s = h.rsplit('/', 1)[-1]
        if s not in seen:
            seen.add(s)
            slugs.append(s)
    return slugs


# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('json_path', nargs='?', default='pokedex.json')
    ap.add_argument('--range', nargs=2, type=int, metavar=('START', 'END'),
                    help='only process these national dex numbers (inclusive)')
    ap.add_argument('--out', help='write here instead of overwriting the input')
    ap.add_argument('--dry-run', action='store_true', help='print changes, write nothing')
    ap.add_argument('--delay', type=float, default=2.0, help='seconds between requests (default 2)')
    ap.add_argument('--cache-dir', default='.cache/pokemondb')
    ap.add_argument('--no-cache', action='store_true', help='ignore cached pages and refetch')
    ap.add_argument('--force', action='store_true', help='re-scrape entries that already have a gender')
    args = ap.parse_args()

    with open(args.json_path, encoding='utf-8') as fp:
        data = json.load(fp)

    max_num = max(int(e['number']) for e in data.values() if e.get('number'))
    lo, hi = args.range or (1, max_num)

    def in_range(entry):
        try:
            return lo <= int(entry.get('number', 0)) <= hi
        except ValueError:
            return False

    # dex number -> keys of non-derived entries (base forms AND regional forms)
    by_num = defaultdict(list)
    for key, entry in data.items():
        if not is_derived(key) and in_range(entry):
            by_num[int(entry['number'])].append(key)

    # only fetch pages that still have something to fill in
    wanted = sorted(n for n, keys in by_num.items()
                    if args.force or any('gender' not in data[k] for k in keys))

    fetcher = Fetcher(args.cache_dir, args.delay, use_cache=not args.no_cache)
    slugs = national_slugs(fetcher)
    print(f'{len(wanted)} pages to fetch (#{lo:04d}-#{hi:04d}), ~{args.delay}s apart')

    # --- pass 1: base forms and regional forms, straight from their own tab ---
    failures = 0
    for num in tqdm(wanted):
        if num > len(slugs):
            tqdm.write(f'! no slug for #{num:04d}')
            continue
        slug = slugs[num - 1]
        try:
            html = fetcher.get(f'/pokedex/{slug}', slug)
            failures = 0
        except (requests.RequestException, RuntimeError, PermissionError) as e:
            tqdm.write(f'! {slug}: {e}')
            failures += 1
            if failures >= 3:
                raise SystemExit('3 failures in a row, stopping so we do not hammer the site.')
            continue

        poke_soup = BeautifulSoup(html, 'html.parser')
        for form, gender in gender_by_form(poke_soup):
            key = form_key(slug, form)
            entry = data.get(key)
            if entry is None:
                if not is_derived(key):
                    tqdm.write(f'? {slug}: tab "{form}" -> "{key}" not in pokedex.json')
                continue
            if int(entry.get('number', num)) != num:
                tqdm.write(f'! {key}: json number {entry["number"]} != page number {num}, skipped')
                continue
            if gender is None:
                tqdm.write(f'! {key}: could not read gender')
                continue
            entry['gender'] = gender
            tqdm.write(f'{key}: {gender}')

    # --- pass 2: megas / primals / gmax inherit from their base form ---
    for key, entry in data.items():
        if not (is_derived(key) and in_range(entry)):
            continue
        parents = entry.get('evolves_from') or []
        base = data.get(parents[0]) if parents else None
        if base is None or 'gender' not in base:
            print(f'! {key}: base form has no gender to inherit')
            continue
        entry['gender'] = base['gender']
        print(f'{key}: {base["gender"]} (from {parents[0]})')

    missing = [k for k, e in data.items() if in_range(e) and 'gender' not in e]
    for key in missing:
        print(f'! {key}: still missing gender')

    if args.dry_run:
        print('Dry run, nothing written (fetched pages are cached for the real run).')
        return
    with open(args.out or args.json_path, 'w', encoding='utf-8') as fp:
        json.dump(data, fp, indent=4, ensure_ascii=False)
        fp.write('\n')
    print('Saved.')


if __name__ == '__main__':
    main()