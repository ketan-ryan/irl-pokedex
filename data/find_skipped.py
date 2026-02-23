from pathlib import Path

label_dir = Path("H:/.shortcut-targets-by-id/18RzIobxE486PBQWMYukMM8r9PjTDbWd5/IRL Pokedex/annotations/tcg_flattened")
all_labels = label_dir.glob('*.txt')
label_stems = [label.stem for label in all_labels if label.name != 'train.txt']
print(len(label_stems))

src_images_path = Path("H:/.shortcut-targets-by-id/18RzIobxE486PBQWMYukMM8r9PjTDbWd5/IRL Pokedex/PokemonTCG")
all_images = src_images_path.rglob('*.png')
all_stems = [img.stem for img in all_images]
print(len(all_stems))

imgs_set = set(all_stems)
labels_set = set(label_stems)

not_annotated = imgs_set - labels_set
print(not_annotated)