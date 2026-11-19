#!/usr/bin/env python3
"""Reorganize sprite folders to match pokedex.json names.

    0003venusar/exclude/poke_capture_0003_000_mf_g_00000000_f_r.png
 -> Venusaur/shiny/gmax/poke_capture_0003_000_mf_g_00000000_f_r.png

    electrode (hisuian)/HOME0101H_s.png
 -> Hisuian Electrode/shiny/HOME0101H_s.png

Usage: python organize_sprites.py ROOT [--json pokedex.json] [--names name_maps.json] [--dry-run]
"""
import argparse
import json
import os
import re
import shutil
from pathlib import Path

DERIVED_PREFIXES = ('Mega ', 'Primal ', 'Gigantamax ')

# poke_capture_<dex>_<form>_<gender>_<kind>_<id>_<f>_<n|r>.png
CAPTURE_RE = re.compile(
    r'^poke_capture_(\d{4})_(\d{3})_([a-z]{2})_([a-z])_\d{8}_[a-z]_([nr])\.png$', re.I)
# HOME0916.png / HOME0916_s.png / HOME0916_f.png / HOME0916_f_s.png
# (an optional form letter may follow the number: HOME0101H_s.png)
HOME_RE = re.compile(r'^HOME(\d{4})([A-Za-z]*)(_f)?(_s)?\.png$', re.I)


def home_stem(m) -> str:
    """Number + form letter, so each form is matched against its own female variant."""
    return (m[1] + m[2]).lower()


def safe_name(key: str) -> str:
    """Strip characters Windows can't have in folder names ('Type: Null' -> 'Type Null')."""
    return re.sub(r'[<>:"/\\|?*]', '', key).rstrip('. ')


def classify(path: Path, in_exclude: bool, female_stems: set):
    """Return (is_shiny, [subfolders]) or None if the filename isn't recognised."""
    if m := CAPTURE_RE.match(path.name):
        _, _, gender, kind, shiny = m.groups()
        parts = []
        if kind.lower() == 'g':
            parts.append('gmax')
        elif in_exclude:
            parts.append('mega')
        if gender.lower() == 'fd':
            parts.append('female')
        elif gender.lower() == 'md':
            parts.append('male')
        return shiny.lower() == 'r', parts
    if not in_exclude and (m := HOME_RE.match(path.name)):
        parts = []
        if m[3]:
            parts.append('female')
        elif home_stem(m) in female_stems:  # a female variant exists, so this is the male one
            parts.append('male')
        return bool(m[4]), parts
    return None


def resolve(folder: Path, data, canon, name_map, by_num):
    """Return (pokedex key, None) or (None, error message)."""
    m = re.match(r'^(\d{4})\s*(.*)$', folder.name)
    num, raw = (m[1], m[2].strip()) if m else (None, folder.name.strip())

    key = None
    for cand in (name_map.get(raw.lower()), raw):  # mapped name first, then the raw name
        if cand and cand.lower() in canon:
            key = canon[cand.lower()]
            break

    if key is None and num:  # misspelled/unknown name: fall back to the dex number
        keys = by_num.get(num, [])
        if len(keys) == 1:
            key = keys[0]
        elif keys:
            return None, f'"{raw}" not found and #{num} is ambiguous: {keys}'
    if key is None:
        return None, f'"{raw}" not found in pokedex.json or name_maps.json'

    if num and data[key].get('number') != num:
        return None, f'matched "{key}" but its number is {data[key].get("number")}, not {num}'
    return key, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('root')
    ap.add_argument('--json', default='pokedex.json')
    ap.add_argument('--names', default='name_maps.json')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    root = Path(args.root)
    data = json.loads(Path(args.json).read_text(encoding='utf-8'))
    name_map = {k.lower(): v for k, v in
                json.loads(Path(args.names).read_text(encoding='utf-8')).items()}
    canon = {k.lower(): k for k in data}  # case-insensitive key lookup
    by_num = {}
    for k, e in data.items():
        if not k.startswith(DERIVED_PREFIXES) and e.get('number'):
            by_num.setdefault(e['number'], []).append(k)

    moves, folders = [], []
    for folder in sorted(p for p in root.iterdir() if p.is_dir()):
        files = [(f, False) for f in sorted(folder.glob('*.png'))]
        female_stems = {home_stem(m) for f, ex in files
                if not ex and (m := HOME_RE.match(f.name)) and m[3]}
        exclude = folder / 'exclude'
        if exclude.is_dir():
            files += [(f, True) for f in sorted(exclude.glob('*.png'))]
        if not files:
            continue  # already organized (or empty)

        key, err = resolve(folder, data, canon, name_map, by_num)
        if err:
            print(f'! {folder.name}: {err}, skipped')
            continue

        target = root / safe_name(key)
        print(f'{folder.name}  ->  {target.name}  ({len(files)} files)')
        for f, in_ex in files:
            result = classify(f, in_ex, female_stems)
            if result is None:
                print(f'  ? unrecognised, left in place: {f.relative_to(root)}')
                continue
            shiny, parts = result
            dst = target.joinpath('shiny' if shiny else 'base', *parts, f.name)
            moves.append((f, dst))
        folders.append((folder, target))

    print(f'\n{len(moves)} files to move')
    if args.dry_run:
        for src, dst in moves[:40]:
            print(f'  {src.relative_to(root)}  ->  {dst.relative_to(root)}')
        if len(moves) > 40:
            print(f'  ... and {len(moves) - 40} more')
        print('Dry run, nothing changed.')
        return

    for src, dst in moves:
        if dst.exists():
            print(f'  = already exists, skipping: {dst.relative_to(root)}')
            continue
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(src, dst)

    # clean up: drop empty exclude/ folders and the old (differently named) folders
    for folder, target in folders:
        ex = folder / 'exclude'
        if ex.is_dir() and not any(ex.iterdir()):
            ex.rmdir()
        if folder.exists() and folder.name != target.name:
            if target.exists() and os.path.samefile(folder, target):
                # case-only rename on a case-insensitive filesystem (venusaur -> Venusaur)
                tmp = folder.with_name(folder.name + '.tmp_rename')
                folder.rename(tmp)
                tmp.rename(target)
            elif not any(folder.iterdir()):
                folder.rmdir()
    print('Done.')


if __name__ == '__main__':
    main()