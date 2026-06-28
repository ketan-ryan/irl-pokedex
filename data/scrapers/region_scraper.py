import json
import requests
from bs4 import BeautifulSoup
from pathlib import Path
from playwright.sync_api import sync_playwright
import time

url = "https://rankedboost.com/pokemon/pokedex/?gen="
dex_path = Path('pokedex/assets/resources/pokedex.json')
regions = ['Kanto', 'Johto', 'Hoenn', 'Sinnoh', 'Unova', 'Kalos', 'Alola', 'Galar', 'Paldea']
with open(dex_path, 'r', encoding='utf-8') as fp:
    data = json.load(fp)

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()
    
    for i in range(1, 10):
        query_url = "https://rankedboost.com/pokemon/pokedex/?gen=" + str(i)
        page.goto(query_url)

        # Scroll to bottom repeatedly until no new content loads
        last_height = 0
        while True:
            page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
            time.sleep(1.5)  # wait for new content to load
            
            new_height = page.evaluate("document.body.scrollHeight")
            if new_height == last_height:
                break  # nothing new loaded, we're done
            last_height = new_height

        html = page.content()
        soup = BeautifulSoup(html, 'html.parser')

        for mon in soup.find("div", class_="pdex-results").children:
            name = mon["data-name"]
            cap_name = name.title()
            if cap_name not in data:
                print(f'Skipping pokemon {cap_name}')
                continue

            dex_data = data[cap_name]
            if 'region' not in dex_data:
                dex_data['region'] = regions[i - 1]
    browser.close()

with open(dex_path, 'w', encoding='utf-8') as fp:
    json.dump(data, fp, indent=4, ensure_ascii=False)
