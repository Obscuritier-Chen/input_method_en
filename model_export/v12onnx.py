import sys
from pathlib import Path

current_file = Path(__file__).resolve()
outer_folder = current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import torch
import onnx
import onnxruntime as ort
import numpy as np

from structure.completion_model import CompletionModel
from structure.model_config import ModelConfig
from structure.ch_vocabulary import CharacterVocabulary
from structure.word_vocabulary import WordVocabulary


# ==========================
# 配置：按你实际路径调整
# ==========================

CHECKPOINT_PATH = "./model/results/models/best.pt"
VOCAB_PATH = "./model/datasets/vocabulary/word2id.json"

OUTPUT_PATH = "./model_export/output/completion_model_v1.onnx"

# 导出用的 dummy 输入尺寸，随便取一个能跑通的合理值即可，
# 实际推理时靠 dynamic_axes 支持任意长度
DUMMY_BATCH = 1
DUMMY_CONTEXT_LEN = 16
DUMMY_PREFIX_LEN = 6

OPSET_VERSION = 17


def main():

    Path(OUTPUT_PATH).parent.mkdir(parents=True, exist_ok=True)

    # ==========================
    # 1. 还原模型结构与训练时一致
    # ==========================

    char_vocab = CharacterVocabulary()
    word_vocab = WordVocabulary(VOCAB_PATH)

    config = ModelConfig()
    config.word_vocab_size = len(word_vocab)
    config.char_vocab_size = len(char_vocab)

    model = CompletionModel(config)

    checkpoint = torch.load(CHECKPOINT_PATH, map_location="cpu")
    model.load_state_dict(checkpoint["model_state_dict"])

    model.eval()

    # ==========================
    # 2. 构造 dummy 输入
    # ==========================
    # 注意：context_mask 的 dtype / 语义（True=有效 还是 True=padding）
    # 必须和训练时 embedding_builder / transformer_encoder 内部的用法完全一致，
    # 这里先假设是 torch.bool，1(True)=有效 token，0(False)=padding，
    # 如果你的实现相反或者是 float/long，需要按实际情况调整下面这行

    context_ids = torch.randint(
        low=0,
        high=config.word_vocab_size,
        size=(DUMMY_BATCH, DUMMY_CONTEXT_LEN),
        dtype=torch.long
    )

    prefix_ids = torch.randint(
        low=0,
        high=config.char_vocab_size,
        size=(DUMMY_BATCH, DUMMY_PREFIX_LEN),
        dtype=torch.long
    )

    context_mask = torch.ones(
        (DUMMY_BATCH, DUMMY_CONTEXT_LEN),
        dtype=torch.bool
    )

    dummy_inputs = (context_ids, prefix_ids, context_mask)

    # ==========================
    # 3. 导出 ONNX
    # ==========================

    torch.onnx.export(

        model,

        dummy_inputs,

        OUTPUT_PATH,

        input_names=["context_ids", "prefix_ids", "context_mask"],

        output_names=["logits"],

        dynamic_axes={
            "context_ids": {0: "batch", 1: "context_len"},
            "prefix_ids": {0: "batch", 1: "prefix_len"},
            "context_mask": {0: "batch", 1: "context_len"},
            "logits": {0: "batch"},
        },

        opset_version=OPSET_VERSION,

        do_constant_folding=True,

    )

    print(f"Exported to {OUTPUT_PATH}")

    # ==========================
    # 4. 校验：ONNX 结构合法性
    # ==========================

    onnx_model = onnx.load(OUTPUT_PATH)
    onnx.checker.check_model(onnx_model)

    print("ONNX model structure check passed.")

    # ==========================
    # 5. 校验：数值一致性（PyTorch vs ONNX Runtime）
    # ==========================

    with torch.no_grad():
        torch_output = model(*dummy_inputs).numpy()

    session = ort.InferenceSession(
        OUTPUT_PATH,
        providers=["CPUExecutionProvider"]
    )

    onnx_output = session.run(
        ["logits"],
        {
            "context_ids": context_ids.numpy(),
            "prefix_ids": prefix_ids.numpy(),
            "context_mask": context_mask.numpy(),
        }
    )[0]

    max_diff = np.abs(torch_output - onnx_output).max()

    print(f"Max abs diff (torch vs onnx): {max_diff:.8f}")

    if max_diff < 1e-4:
        print("Numerical check passed.")
    else:
        print("WARNING: numerical diff is larger than expected, please inspect.")


if __name__ == "__main__":
    main()