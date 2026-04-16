import os
import json
import re
import requests
import time

JSONL_FILE = "download.jsonl"
DATASET_ROOT = "MSML-612-Project\poke-data\ProjectPokemon"  # <-- CHANGE THIS
LOG_FILE = "download_log.txt"

DRY_RUN = False  # 🔥 set to False to actually download

HEADERS = {
    "User-Agent": "Mozilla/5.0 (compatible; PokemonDownloader/1.0)"
}


# -----------------------------
# Logging
# -----------------------------
def log(msg):
    print(msg)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(msg + "\n")


# -----------------------------
# Extract HOME ID
# -----------------------------
def extract_home_id(filename):
    match = re.search(r'HOME(\d{4})', filename)
    if match:
        return match.group(1)
    return None


# -----------------------------
# Map ID -> folder path
# -----------------------------
def build_folder_map():
    mapping = {}

    for folder in os.listdir(DATASET_ROOT):
        match = re.match(r'(\d{4})', folder)
        if match:
            mapping[match.group(1)] = os.path.join(DATASET_ROOT, folder)

    return mapping


# -----------------------------
# Download image
# -----------------------------
def download_image(url, save_path):
    try:
        r = requests.get(url, headers=HEADERS, timeout=30)
        r.raise_for_status()

        with open(save_path, "wb") as f:
            f.write(r.content)

        return True

    except Exception as e:
        log(f"ERROR downloading {url}: {e}")
        return False


# -----------------------------
# Main
# -----------------------------
def main():
    folder_map = build_folder_map()

    total = 0
    skipped = 0
    downloaded = 0

    with open(JSONL_FILE, "r", encoding="utf-8") as f:
        for line in f:
            try:
                obj = json.loads(line)
                url = obj.get("image_url", "")
                filename = url.split("/")[-1]

                poke_id = extract_home_id(filename)

                if not poke_id:
                    continue

                # 🔥 ONLY > 898
                # if int(poke_id) <= 898:
                #     continue

                folder = folder_map.get(poke_id)

                if not folder:
                    log(f"No folder found for {poke_id}")
                    continue

                save_path = os.path.join(folder, filename)

                total += 1

                # skip if already exists
                if os.path.exists(save_path):
                    skipped += 1
                    continue

                if DRY_RUN:
                    log(f"[DRY RUN] Would download: {url} -> {save_path}")
                    continue

                success = download_image(url, save_path)

                if success:
                    downloaded += 1
                    log(f"Downloaded: {filename}")

                time.sleep(0.2)  # be polite

            except Exception as e:
                log(f"ERROR processing line: {e}")

    print("\n====================")
    print(f"Total candidates (>898): {total}")
    print(f"Already exists (skipped): {skipped}")
    print(f"Downloaded: {downloaded}")

    if DRY_RUN:
        print("\n⚠️ DRY RUN ENABLED — no files downloaded.")
    else:
        print("\n✅ Download complete.")


if __name__ == "__main__":
    main()