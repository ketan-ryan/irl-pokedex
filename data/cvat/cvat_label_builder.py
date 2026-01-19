import json
import os

script_dir = os.path.dirname(os.path.abspath(__file__))

classes = []
with open(os.path.abspath(os.path.join(script_dir, '..', '..', '..', 'all_pokemon_safe.json')), "r") as fp:
    _in = json.load(fp)
    for _class in _in:
        out_j = {
            "name": _class,
            "type": "any",
            "attributes": []
        }
        classes.append(out_j)

with open('cvat_labels.json', 'w') as fp:
    json.dump(classes, fp)