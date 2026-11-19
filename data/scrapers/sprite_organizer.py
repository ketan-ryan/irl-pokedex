#!/usr/bin/env python3
"""Copy Mega / Primal / Gigantamax sprites out of each Pokémon's `exclude`
folder into new sibling folders (one per form), renamed to:
    0006 Charizard Mega Y.png        -> Mega Charizard Y/Mega Charizard Y.png
    0383 groudon primal 2.png        -> Primal Groudon/Primal Groudon 2.png
    0818Inteleon-Gigantamax.png      -> Gigantamax Inteleon/Gigantamax Inteleon.png
The `exclude` folder is never modified.

Filename tolerance: the separator after the dex number is optional, words may be
separated by spaces, hyphens or underscores, case is ignored, and an optional
" n" suffix is allowed on megas, primals and gmax. Base names are matched
case-insensitively against pokedex.json keys so the output uses the proper casing.

pokedex.json is also updated, once per form (numbered duplicates add nothing):
  * base entry gets  "mega": [<each mega/primal name>]  (merged with any existing list)
  * base entry gets  "gmax": "Gigantamax <Name>"
  * each mega/primal/gmax form without a key gets a new entry right after its
    base form: a copy of the base minus "gmax", "mega" and "region", with
    dex_entries = {}, evolves_from = [base], evolves_into = []
  * existing mega/primal entries just get the base added to evolves_from if missing

Usage: python organize_forms.py [root] [--json pokedex.json] [--dry-run]
"""
import copy
import json
import re
import shutil
import sys
from pathlib import Path

SEP = r"[\s\-_]+"   # between words
NUM = r"^\d{4}[\s\-_]*(?P<name>.+?)"  # dex number, optional separator, base name
N_SUFFIX = rf"(?:{SEP}(?P<n>\d+))?"

GMAX_RE = re.compile(rf"{NUM}{SEP}Gigantamax{N_SUFFIX}\.png$", re.I)
MEGA_RE = re.compile(rf"{NUM}{SEP}Mega(?:{SEP}(?P<variant>[XYZ]))?{N_SUFFIX}\.png$", re.I)
PRIMAL_RE = re.compile(rf"{NUM}{SEP}Primal{N_SUFFIX}\.png$", re.I)


def classify(filename: str, canon: dict):
    """Return (kind, base_name, new_filename, form_name) or None if no match.
    kind is 'gigantamax' or 'mega' (primals count as 'mega').
    form_name is the JSON key for the form (no numeric suffix)."""
    def base(raw):
        return canon.get(raw.strip().lower(), raw.strip())

    if m := GMAX_RE.match(filename):
        b = base(m["name"])
        n = f" {m['n']}" if m["n"] else ""
        form = f"Gigantamax {b}"
        return "gigantamax", b, f"{form}{n}.png", form
    if m := MEGA_RE.match(filename):
        b = base(m["name"])
        v = f" {m['variant'].upper()}" if m["variant"] else ""
        n = f" {m['n']}" if m["n"] else ""
        form = f"Mega {b}{v}"
        return "mega", b, f"{form}{n}.png", form
    if m := PRIMAL_RE.match(filename):
        b = base(m["name"])
        n = f" {m['n']}" if m["n"] else ""
        form = f"Primal {b}"
        return "mega", b, f"{form}{n}.png", form
    return None


def insert_after_family(data: dict, base_name: str, key: str, entry: dict):
    """Insert `entry` after the base form and any of its existing mega/gmax entries,
    so forms stay grouped and in order."""
    base = data[base_name]
    family = {base_name, *base.get("mega", [])}
    if base.get("gmax"):
        family.add(base["gmax"])
    last = max(i for i, k in enumerate(data) if k in family)
    items = list(data.items())
    items.insert(last + 1, (key, entry))
    data.clear()
    data.update(items)


def make_form_entry(base_name: str, base: dict):
    entry = copy.deepcopy(base)
    for k in ("gmax", "mega", "region"):
        entry.pop(k, None)
    entry["dex_entries"] = {}
    entry["evolves_from"] = [base_name]
    entry["evolves_into"] = []
    entry["base"] = False
    return entry


def update_json(data: dict, base_name: str, megas: list, has_gmax: bool):
    """Mutate `data` in place. Returns a list of messages describing what changed."""
    base = data.get(base_name)
    if base is None:
        return [f"  ! '{base_name}' not found in pokedex.json (forms skipped)"]
    msgs = []

    # --- mega / primal ---
    if megas:
        current = list(base.get("mega", []))
        merged = current + [m for m in megas if m not in current]
        if merged != current:
            base["mega"] = merged
            msgs.append(f"  json: {base_name}.mega = {merged}")

        # megas/primals live in the "mega" field, so drop them from evolves_into
        if "evolves_into" in base:
            old = base["evolves_into"]
            kept = [e for e in old if e not in merged]
            if kept:
                base["evolves_into"] = kept
            else:
                del base["evolves_into"]
            if kept != old or not kept:
                msgs.append(f"  json: {base_name}.evolves_into -> {kept or 'removed'}")

        for mega_name in megas:
            if mega_name in data:
                form = data[mega_name]
                if form.get("base") is not False:
                    form["base"] = False
                    msgs.append(f"  json: '{mega_name}'.base = false")
                evo = form.setdefault("evolves_from", [])
                if base_name not in evo:
                    evo.append(base_name)
                    msgs.append(f"  json: '{mega_name}'.evolves_from += '{base_name}'")
            else:
                insert_after_family(data, base_name, mega_name, make_form_entry(base_name, base))
                msgs.append(f"  json: added entry '{mega_name}'")
    # --- gigantamax ---
    if has_gmax:
        gmax_name = f"Gigantamax {base_name}"
        if base.get("gmax") != gmax_name:
            base["gmax"] = gmax_name
            msgs.append(f"  json: {base_name}.gmax = '{gmax_name}'")
        if gmax_name not in data:
            insert_after_family(data, base_name, gmax_name, make_form_entry(base_name, base))
            msgs.append(f"  json: added entry '{gmax_name}'")
        elif data[gmax_name].get("base") is not False:
            data[gmax_name]["base"] = False
            msgs.append(f"  json: '{gmax_name}'.base = false")
    return msgs


def main():
    argv = sys.argv[1:]
    dry_run = "--dry-run" in argv
    json_path = Path("pokedex.json")
    if "--json" in argv:
        i = argv.index("--json")
        json_path = Path(argv[i + 1])
        del argv[i : i + 2]
    args = [a for a in argv if not a.startswith("--")]
    root = Path(args[0] if args else "registering_sprites")

    if not root.is_dir():
        sys.exit(f"Root folder not found: {root}")

    data = None
    if json_path.is_file():
        data = json.loads(json_path.read_text(encoding="utf-8"))
    else:
        print(f"Warning: {json_path} not found; skipping JSON updates.")

    # lowercase name -> proper pokedex.json key (for case-insensitive base lookup)
    canon = {k.lower(): k for k in data} if data else {}

    copied = skipped = 0
    for poke_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        exclude = poke_dir / "exclude"
        if not exclude.is_dir():
            continue

        # base name -> {"megas": [names], "gmax": bool}; JSON is updated once per base
        forms = {}

        for src in sorted(exclude.glob("*.png")):
            result = classify(src.name, canon)
            if result is None:
                print(f"  ? unmatched: {src}")
                continue

            kind, base_name, filename, form_name = result
            dest_dir = root / form_name          # sibling of the base folder
            dest = dest_dir / filename

            if dest.exists():
                print(f"  = exists, skipping copy: {dest}")
                skipped += 1
            else:
                print(f"{src.relative_to(root)}  ->  {dest.relative_to(root)}")
                if not dry_run:
                    dest_dir.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(src, dest)
                copied += 1

            info = forms.setdefault(base_name, {"megas": [], "gmax": False})
            if kind == "gigantamax":
                info["gmax"] = True
            elif form_name not in info["megas"]:
                info["megas"].append(form_name)

        if data is not None:
            for base_name, info in forms.items():
                for msg in update_json(data, base_name, info["megas"], info["gmax"]):
                    print(msg)

    if data is not None and not dry_run:
        json_path.write_text(
            json.dumps(data, indent=4, ensure_ascii=False) + "\n", encoding="utf-8"
        )

    action = "Would copy" if dry_run else "Copied"
    print(f"\n{action} {copied} file(s), skipped {skipped}.")


if __name__ == "__main__":
    main()