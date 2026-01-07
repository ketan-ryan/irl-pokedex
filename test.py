import json

classes = []
with open('classes_cleaned.json', 'r') as fp:
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