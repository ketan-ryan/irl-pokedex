import json
import re
from urllib.parse import urlparse, urlunparse

GENERIC_SEQ_RE = re.compile(r"(.*?)([-]?)(\d+)(\.jpg)$")

def generate_sequential_candidates(urls, max_index=15):
    groups = {}

    for url in urls:
        path = urlparse(url).path
        match = GENERIC_SEQ_RE.search(path)
        if not match:
            continue

        base_text, sep, idx, ext = match.groups()
        idx = int(idx)

        # Store by the combination of base + separator to preserve style
        key = base_text + sep
        groups.setdefault(key, {
            "ext": ext,
            "example_url": url,
            "sep": sep,
            "indices": set()
        })
        groups[key]["indices"].add(idx)

    candidates = []

    for key, info in groups.items():
        parsed = urlparse(info["example_url"])
        path = parsed.path
        query = parsed.query

        for i in range(1, max_index + 1):
            if i in info["indices"]:
                continue

            new_idx = f"{i:02d}"  # zero-padded 2 digits
            new_filename = f"{key}{new_idx}{info['ext']}"

            # replace only the last number in the path
            new_path = GENERIC_SEQ_RE.sub(new_filename, path)

            new_url = urlunparse((
                parsed.scheme,
                parsed.netloc,
                new_path,
                "",
                query,
                ""
            ))

            candidates.append(new_url)

    return candidates

with open("pokemon_plush_images.json", "r", encoding="utf-8") as f:
    data = json.load(f)

seq_output = {}

for pokemon, urls in data.items():
    seq_candidates = generate_sequential_candidates(urls)

    if seq_candidates:
        seq_output[pokemon] = seq_candidates

with open("pokemon_plush_images_seq_candidates.json", "w", encoding="utf-8") as f:
    json.dump(seq_output, f, indent=2, ensure_ascii=False)

print(f"Saved {len(seq_output)} Pokémon with sequential candidate URLs.")

with open("rotom_plushdex_plush_cleaned.json", "r", encoding="utf-8") as f:
    data = json.load(f)

seq_output = {}

for pokemon, urls in data.items():
    seq_candidates = generate_sequential_candidates(urls)

    if seq_candidates:
        seq_output[pokemon] = seq_candidates

with open("rotom_plushdex_cleaned_seq_candidates.json", "w", encoding="utf-8") as f:
    json.dump(seq_output, f, indent=2, ensure_ascii=False)

print(f"Saved {len(seq_output)} Pokémon with sequential candidate URLs.")