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

from model.structure.completion_model_v3 import CompletionModelV3
from model.structure.model_config import ModelConfig
from model.structure.word_vocabulary import WordVocabulary


CHECKPOINT_PATH = "./model/results/models/best_v3.pt"
VOCAB_PATH = "./model/datasets/vocabulary/word2id.json"

OUTPUT_PATH = "./model_export/output/completion_model_v3.onnx"

DUMMY_BATCH = 1
DUMMY_CONTEXT_LEN = 16
DUMMY_CANDIDATE_COUNT = 32

OPSET_VERSION = 17


def main():

    Path(OUTPUT_PATH).parent.mkdir(parents=True, exist_ok=True)

    word_vocab = WordVocabulary(VOCAB_PATH)

    config = ModelConfig()
    config.word_vocab_size = len(word_vocab)

    model = CompletionModelV3(config)

    checkpoint = torch.load(CHECKPOINT_PATH, map_location="cpu")
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()

    context_ids = torch.randint(
        0, config.word_vocab_size, (DUMMY_BATCH, DUMMY_CONTEXT_LEN), dtype=torch.long
    )

    candidate_ids = torch.randint(
        0, config.word_vocab_size, (DUMMY_BATCH, DUMMY_CANDIDATE_COUNT), dtype=torch.long
    )

    candidate_mask = torch.ones(
        (DUMMY_BATCH, DUMMY_CANDIDATE_COUNT), dtype=torch.bool
    )

    dummy_inputs = (context_ids, candidate_ids, candidate_mask)

    torch.onnx.export(

        model,

        dummy_inputs,

        OUTPUT_PATH,

        input_names=["context_ids", "candidate_ids", "candidate_mask"],

        output_names=["logits"],

        dynamic_axes={
            "context_ids": {0: "batch", 1: "context_len"},
            "candidate_ids": {0: "batch", 1: "candidate_count"},
            "candidate_mask": {0: "batch", 1: "candidate_count"},
            "logits": {0: "batch", 1: "candidate_count"},
        },

        opset_version=OPSET_VERSION,

        do_constant_folding=True,

    )

    print(f"Exported to {OUTPUT_PATH}")

    # -------------------------
    # ONNX 结构校验
    # -------------------------

    onnx_model = onnx.load(OUTPUT_PATH)
    onnx.checker.check_model(onnx_model)

    print("ONNX model structure check passed.")

    # -------------------------
    # 数值一致性校验
    # -------------------------

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
            "candidate_ids": candidate_ids.numpy(),
            "candidate_mask": candidate_mask.numpy(),
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