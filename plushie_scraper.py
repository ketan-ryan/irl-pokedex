from bs4 import BeautifulSoup
from playwright.async_api import async_playwright
from playwright.sync_api import sync_playwright, Page, Browser

import asyncio
from pathlib import Path
import random
import re
import requests
from tqdm import tqdm

plushie_url = "https://myfigurecollection.net/?tab=search&rootId=1&status=-1&categoryId=-1&contentLevel=-1&excludeContentLevel=0&orEntries%5B%5D=241&domainId=-1&noReleaseDate=0&releaseTypeId=0&ratingId=0&isCastoff=0&hasBootleg=0&tagId=0&noBarcode=0&clubId=0&excludeClubId=0&listId=0&isDraft=0&year=2025&month=1&separator=0&sort=popularity&order=asc&output=2&current=categoryId&page=2&_tb=item"
mapping = {}
with open('pokemon to japanese names.csv', 'r', encoding='utf-8') as fp:
    for line in fp.readlines():
        items = line.split(',')
        mapping[items[-1].strip().lower()] = items[2]

# regex to find partial matches
# example: searching for "alolan vulpix" (not in dict) will hit on "vulpix" (in dict)
pattern = re.compile(
    "|".join(re.escape(k) for k in sorted(mapping, key=len, reverse=True)),
    re.IGNORECASE
)

async def download_page(playwright, page: Page = None, browser: Browser = None, url = None):
    if page:
        await page.close()
        await browser.close()
    
    browser = await playwright.chromium.launch(headless=False)
    page = await browser.new_page()

    await page.goto(url)
    await page.wait_for_timeout(2000 + random.randint(0, 1000))
    
    html = await page.content()
    soup = BeautifulSoup(html, 'html.parser')
    current_page = soup.find(class_="item-icons")

    for span in tqdm(current_page.children):
        a = span.find_next("a")
        img = a.find("img") if a else None

        alt = img.get('alt')
        components = [a.strip().lower() for a in alt.split('-')]

        m = pattern.search(components[1])
        val = mapping[m.group()] if m else None
        
        # not a pokemon, don't download it
        if not val: continue

        src = img.get('src')
        src_full = src.replace('/items/0/', '/items/1/')
        src_filename = src.split('/')[-1]
        out_path = Path(f'plushies/{val}-{src_filename}')

        # no need to re-download
        if out_path.exists(): continue
        response = requests.get(src_full)
        
        if response.status_code == 200:
            with open(out_path, 'wb') as fp:
                fp.write(response.content)
        else:
            print('Failed to dl image with code', response.status_code)

    pages = soup.find(class_="results-count-pages")
    kids = list(pages.children)
    for idx, a in enumerate(pages.children):
        if 'nav-current' in a.get('class'):
            if idx == len(kids) - 1:
                return
            print(a.get_text())
            if a.get_text() == '1':
                return
            await download_page(playwright, page, browser, kids[idx + 1].get('href'))


async def main():
    async with async_playwright() as playwright:
        await download_page(playwright, page=None, browser=None, url=plushie_url)

asyncio.run(main())
