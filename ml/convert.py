from ultralytics import YOLO


model = YOLO('pokedex/assets/yolo26n-cls.pt')
model.export(format='onnx')