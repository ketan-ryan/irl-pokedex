import os
import re
import json
import time
import requests
from urllib.parse import urljoin
from concurrent.futures import ThreadPoolExecutor, as_completed

BASE_URL = "https://www.flickr.com"
REQUEST_DELAY = 5  # seconds between requests
MAX_WORKERS = 8
MISSING_LOG = "missing_originals.txt"
PER_PAGE = 25  # albums per page
with open('secrets.txt', 'r') as fp:
    API_KEY = fp.readline().strip()
# API_KEY =   # public key from Flickr site

# XHR ALBUM FETCHING
def get_album_links_xhr(user_id):
    """Fetch all albums via Flickr's photosets.getList XHR endpoint."""
    base_url = "https://api.flickr.com/services/rest"
    albums = []
    page = 1

    while True:
        params = {
            "method": "flickr.photosets.getList",
            "user_id": user_id,
            "api_key": API_KEY,
            "format": "json",
            "nojsoncallback": 1,
            "per_page": PER_PAGE,
            "page": page
        }
        resp = requests.get(base_url, params=params, headers={"User-Agent": "Mozilla/5.0"})
        data = resp.json()
        if "photosets" not in data or "photoset" not in data["photosets"]:
            break

        albums.extend(data["photosets"]["photoset"])
        if page >= data["photosets"]["pages"]:
            break
        page += 1
        time.sleep(1)  # small pause between XHR calls

    # Return album dicts containing id and title
    return [{"id": a["id"], "title": a["title"]["_content"]} for a in albums]

# FETCH HTML
def fetch_html(url):
    time.sleep(REQUEST_DELAY)
    resp = requests.get(url, headers={"User-Agent": "Mozilla/5.0"})
    resp.raise_for_status()
    return resp.text

# PHOTO PARSING
def parse_album(album_url):
    html = fetch_html(album_url)
    json_blob = re.search(r'modelExport:({.*}),\n', html)
    if not json_blob:
        raise ValueError(f"Could not find Flickr JSON in {album_url}")
    data = json.loads(json_blob.group(1))
    album_title = data["main"]["setModel"]["title"].strip().replace("/", "_")
    items = data["main"]["photos"]["_data"]
    photo_data = []
    for item in items:
        original_url = item.get("url_o")
        if original_url:
            url = original_url
            has_original = True
        else:
            size_urls = [v for k, v in item.items() if k.startswith("url_") and v]
            if size_urls:
                url = max(
                    size_urls,
                    key=lambda u: int(re.search(r"_(\d+)\.", u).group(1))
                    if re.search(r"_(\d+)\.", u)
                    else 0,
                )
            else:
                continue
            has_original = False
        title = item.get("title") or str(item.get("id"))
        title = title.strip().replace("/", "_") or str(item.get("id"))
        photo_data.append((title, url, has_original))
    return album_title, photo_data

def get_photostream_photos(user_photostream_url):
    photo_data = []
    page_num = 1
    while True:
        page_url = f"{user_photostream_url}?page={page_num}"
        html = fetch_html(page_url)
        json_blob = re.search(r'modelExport:({.*}),\n', html)
        if not json_blob:
            break
        data = json.loads(json_blob.group(1))
        items = data["main"]["photos"]["_data"]
        if not items:
            break
        for item in items:
            original_url = item.get("url_o")
            if original_url:
                url = original_url
                has_original = True
            else:
                size_urls = [v for k, v in item.items() if k.startswith("url_") and v]
                if size_urls:
                    url = max(
                        size_urls,
                        key=lambda u: int(re.search(r"_(\d+)\.", u).group(1))
                        if re.search(r"_(\d+)\.", u)
                        else 0,
                    )
                else:
                    continue
                has_original = False
            title = item.get("title") or str(item.get("id"))
            title = title.strip().replace("/", "_") or str(item.get("id"))
            photo_data.append((title, url, has_original))
        print(f"Photostream page {page_num}: {len(items)} photos found")
        page_num += 1
    return photo_data

# DOWNLOAD
def download_photo(url, save_path):
    try:
        resp = requests.get(url, stream=True, headers={"User-Agent": "Mozilla/5.0"})
        resp.raise_for_status()
        with open(save_path, "wb") as f:
            for chunk in resp.iter_content(1024):
                f.write(chunk)
        return True, None
    except Exception as e:
        return False, str(e)

# MAIN
def main():
    user_url = input("Enter Flickr user URL (e.g. https://www.flickr.com/photos/username): ").strip()
    user_photostream_url = user_url

    # Extract the user NSID from photostream URL via an initial page fetch
    html = fetch_html(user_url)
    nsid_match = re.search(r'"nsid":"([^"]+)"', html)
    if not nsid_match:
        print("Could not determine user NSID")
        return
    user_id = nsid_match.group(1)

    logged_missing = set()
    if os.path.exists(MISSING_LOG):
        with open(MISSING_LOG, "r", encoding="utf-8") as lf:
            logged_missing = {line.strip() for line in lf}
    log_file = open(MISSING_LOG, "a", encoding="utf-8")

    # Albums
    albums = get_album_links_xhr(user_id)
    print(f"\nFound {len(albums)} albums")
    for album in albums:
        album_id = album["id"]
        album_title = album["title"].strip().replace("/", "_")
        album_url = f"{user_url}/albums/{album_id}"
        print(f"\nProcessing album: {album_title}")
        try:
            _, photos = parse_album(album_url)
        except Exception as e:
            print(f"Failed to parse {album_url}: {e}")
            continue
        os.makedirs(album_title, exist_ok=True)
        tasks = []
        with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
            for title, img_url, has_original in photos:
                filename = os.path.join(album_title, f"{title}.jpg")
                if os.path.exists(filename):
                    if not has_original:
                        log_entry = f"{album_title}/{title}.jpg -> Missing Original"
                        if log_entry not in logged_missing:
                            log_file.write(log_entry + "\n")
                            logged_missing.add(log_entry)
                    continue
                if not has_original:
                    log_entry = f"{album_title}/{title}.jpg -> Missing Original"
                    if log_entry not in logged_missing:
                        log_file.write(log_entry + "\n")
                        logged_missing.add(log_entry)
                tasks.append(executor.submit(download_photo, img_url, filename))
            for future in as_completed(tasks):
                success, error = future.result()
                print(f"   {'SUCCESS: ' if success else 'FAILED: '} Downloaded: {error if error else ''}")

    # Photostream
    print("\nProcessing photostream for images not in albums")
    photostream_photos = get_photostream_photos(user_photostream_url)
    save_folder = "photostream_only"
    os.makedirs(save_folder, exist_ok=True)
    tasks = []
    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        for title, img_url, has_original in photostream_photos:
            filename = os.path.join(save_folder, f"{title}.jpg")
            if os.path.exists(filename):
                if not has_original:
                    log_entry = f"{save_folder}/{title}.jpg -> Missing Original"
                    if log_entry not in logged_missing:
                        log_file.write(log_entry + "\n")
                        logged_missing.add(log_entry)
                continue
            if not has_original:
                log_entry = f"{save_folder}/{title}.jpg -> Missing Original"
                if log_entry not in logged_missing:
                    log_file.write(log_entry + "\n")
                    logged_missing.add(log_entry)
            tasks.append(executor.submit(download_photo, img_url, filename))
        for future in as_completed(tasks):
            success, error = future.result()
            print(f"   {'SUCCESS: ' if success else 'FAILED: '} Downloaded: {error if error else ''}")

    log_file.close()
    print("\nDone! Missing originals logged in missing_originals.txt")

if __name__ == "__main__":
    main()
