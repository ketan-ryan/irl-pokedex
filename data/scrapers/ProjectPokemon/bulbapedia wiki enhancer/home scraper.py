import asyncio
import aiohttp
import json
import os
from bs4 import BeautifulSoup
import time

BASE_URL = "https://archives.bulbagarden.net"
START_URL = "https://archives.bulbagarden.net/wiki/Category:HOME_artwork"

OUTPUT_FILE = "image_urls.jsonl"
LOG_FILE = "scrape_log.txt"
SEEN_FILE = "seen_files.json"

HEADERS = {
    "User-Agent": "Mozilla/5.0 (compatible; PokemonScraper/2.0)"
}

# -----------------------------
# Persistence helpers
# -----------------------------

def log(msg):
    print(msg)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(msg + "\n")


def load_seen():
    if os.path.exists(SEEN_FILE):
        with open(SEEN_FILE, "r", encoding="utf-8") as f:
            return set(json.load(f))
    return set()


def save_seen(seen):
    with open(SEEN_FILE, "w", encoding="utf-8") as f:
        json.dump(list(seen), f)


def save_result(data):
    with open(OUTPUT_FILE, "a", encoding="utf-8") as f:
        f.write(json.dumps(data) + "\n")


# -----------------------------
# Async rate limiter
# -----------------------------
class RateLimiter:
    def __init__(self, delay=0.3):
        self.delay = delay
        self.lock = asyncio.Lock()
        self.last_time = 0

    async def wait(self):
        async with self.lock:
            now = time.time()
            wait_time = self.delay - (now - self.last_time)
            if wait_time > 0:
                await asyncio.sleep(wait_time)
            self.last_time = time.time()


limiter = RateLimiter(delay=0.1)  # safe default


# -----------------------------
# HTTP fetch
# -----------------------------
async def fetch(session, url):
    await limiter.wait()
    async with session.get(url, headers=HEADERS, timeout=30) as resp:
        resp.raise_for_status()
        return await resp.text()


# -----------------------------
# Parse category page
# -----------------------------
def parse_category(html):
    soup = BeautifulSoup(html, "html.parser")

    file_links = []
    for a in soup.select("a.mw-file-description"):
        href = a.get("href")
        if href and href.startswith("/wiki/File:"):
            file_links.append(BASE_URL + href)

    next_page = None
    next_a = soup.find("a", string="next page")
    if next_a and next_a.get("href"):
        next_page = BASE_URL + next_a["href"]

    return file_links, next_page


# -----------------------------
# Parse file page
# -----------------------------
def parse_file_page(html):
    soup = BeautifulSoup(html, "html.parser")
    meta = soup.find("meta", property="og:image")
    if meta:
        return meta.get("content")
    return None


# -----------------------------
# Worker for file pages
# -----------------------------
async def process_file(session, file_url, seen, sem):
    async with sem:
        if file_url in seen:
            return None

        try:
            html = await fetch(session, file_url)
            img_url = parse_file_page(html)

            if img_url:
                data = {
                    "file_page": file_url,
                    "image_url": img_url
                }

                save_result(data)
                seen.add(file_url)

                # ✅ ADD THIS: log full resolution image URL like old script
                log(f"✔ {file_url} -> {img_url}")

                return data

            else:
                log(f"Missing og:image for {file_url}")

        except Exception as e:
            log(f"ERROR {file_url}: {e}")

        return None


# -----------------------------
# Main loop
# -----------------------------
async def main():
    seen = load_seen()
    log(f"Loaded {len(seen)} seen files")

    sem = asyncio.Semaphore(5)  # MAX concurrency (safe)

    async with aiohttp.ClientSession() as session:
        url = START_URL

        while url:
            log(f"\nCategory page: {url}")

            html = await fetch(session, url)
            file_links, next_url = parse_category(html)

            log(f"Found {len(file_links)} files")

            tasks = [
                process_file(session, f, seen, sem)
                for f in file_links
            ]

            # run batch concurrently (bounded)
            await asyncio.gather(*tasks)

            save_seen(seen)

            url = next_url

    log("DONE")


if __name__ == "__main__":
    asyncio.run(main())