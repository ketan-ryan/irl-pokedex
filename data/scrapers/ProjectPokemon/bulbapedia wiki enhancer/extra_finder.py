import os
import json
import re
from collections import defaultdict

JSONL_FILE = "image_urls_dedup.jsonl"
DATASET_ROOT = "MSML-612-Project\poke-data\ProjectPokemon"  # <-- CHANGE THIS
OUTPUT_LOG = "comparison_log.txt"

# -----------------------------
# Extract HOME ID (e.g. 0001)
# -----------------------------
def extract_home_id(filename):
    match = re.search(r'HOME(\d{4})', filename)
    if match:
        return match.group(1)
    return None


# -----------------------------
# Count images from JSONL
# -----------------------------
def count_json_images():
    counts = defaultdict(int)

    with open(JSONL_FILE, "r", encoding="utf-8") as f:
        for line in f:
            try:
                obj = json.loads(line)
                url = obj.get("image_url", "")
                filename = url.split("/")[-1]

                poke_id = extract_home_id(filename)
                if poke_id:
                    counts[poke_id] += 1

            except:
                continue

    return counts


# -----------------------------
# Count images in dataset folders
# -----------------------------
def count_dataset_images():
    counts = defaultdict(int)

    for folder in os.listdir(DATASET_ROOT):
        folder_path = os.path.join(DATASET_ROOT, folder)

        if not os.path.isdir(folder_path):
            continue

        # extract leading 4-digit ID from folder name
        match = re.match(r'(\d{4})', folder)
        if not match:
            continue

        poke_id = match.group(1)

        for file in os.listdir(folder_path):
            if file.lower().endswith(".png"):
                counts[poke_id] += 1

    return counts


# -----------------------------
# Compare and log differences
# -----------------------------
def compare_counts(json_counts, dataset_counts):
    all_ids = set(json_counts.keys()) | set(dataset_counts.keys())

    with open(OUTPUT_LOG, "w", encoding="utf-8") as log:
        for poke_id in sorted(all_ids):
            json_count = json_counts.get(poke_id, 0)
            dataset_count = dataset_counts.get(poke_id, 0)

            if json_count != dataset_count:
                msg = (
                    f"Pokemon {poke_id}: "
                    f"JSON={json_count}, DATASET={dataset_count}, "
                    f"DIFF={json_count - dataset_count}"
                )

                print(msg)
                log.write(msg + "\n")


# -----------------------------
# Main
# -----------------------------
def main():
    print("Counting JSON images...")
    json_counts = count_json_images()

    print("Counting dataset images...")
    dataset_counts = count_dataset_images()

    print("Comparing...")
    compare_counts(json_counts, dataset_counts)

    print(f"\nDone. Results saved to {OUTPUT_LOG}")


if __name__ == "__main__":
    main()