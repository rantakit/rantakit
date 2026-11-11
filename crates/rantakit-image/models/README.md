# Models for the `ml` feature

Experimental, not in release builds. The files are not committed; building with
`--features ml` needs them here under these names.

| File | Size | SHA-256 | Source and license |
|---|---:|---|---|
| `doccornernet.onnx` | 1.98 MB | `525303d5…eb17af` | [DocCornerNet-CoordClass](https://github.com/mapo-t2i/DocCornerNet-CoordClass) MobileNetV2 224 weights (MIT), exported to ONNX opset 17 with tf2onnx (shadow-lab `export_doccornernet.py`) |
| `gcdrnet_gcnet.onnx` | 5.96 MB | `403e576a…767dc3` | [SimdGcdrNet](https://github.com/sdcb/SimdGcdrNet) `gcdrnet_gcnet_sim.onnx` (Apache-2.0), weights from [GCDRNet](https://github.com/ZZZHANG-jx/GCDRNet) (MIT) |
| `gcdrnet_drnet.onnx` | 16.0 MB | `98be60c1…849b94` | SimdGcdrNet `gcdrnet_drnet_sim.onnx`, same licenses |

DocCornerNet's training data (MIDV-500, MIDV-2019, SmartDoc) is licensed
for research use only; whether the weights may ship is still open.
