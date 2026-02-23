from pathlib import Path
import shutil

src_images_path = Path("H:/.shortcut-targets-by-id/18RzIobxE486PBQWMYukMM8r9PjTDbWd5/IRL Pokedex/PokemonTCG")
all_images = src_images_path.rglob('*.png')
all_names = [img.name for img in all_images]

labeled_images_path = Path("H:/.shortcut-targets-by-id/18RzIobxE486PBQWMYukMM8r9PjTDbWd5/IRL Pokedex/annotations/most tcg")
flattened_path = Path(f"{labeled_images_path.parent}/{labeled_images_path.stem}_flattened")

if not flattened_path.exists():
    flattened_path.mkdir()

for img in labeled_images_path.rglob('*'):
    if not img.is_file() or img.suffix == '.ini': continue

    shutil.copy(img, flattened_path / img.name)
