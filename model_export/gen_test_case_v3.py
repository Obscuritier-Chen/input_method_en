import sys
from pathlib import Path

current_file = Path(__file__).resolve()
outer_folder = current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import json
import torch

from model.structure.completion_model_v3 import CompletionModelV3
from model.structure.model_config import ModelConfig
from model.structure.word_vocabulary import WordVocabulary


CHECKPOINT_PATH = "./model/results/models/best_v3.pt"
VOCAB_PATH = "./model/datasets/vocabulary/word2id.json"
OUTPUT_JSON = "./model_export/output/test_case_v3.json"

CONTEXT_LEN = 8
CANDIDATE_COUNT = 32   # 模拟一个前缀对应 32 个候选词的情况


def main():

    torch.manual_seed(42)

    word_vocab = WordVocabulary(VOCAB_PATH)

    config = ModelConfig()
    config.word_vocab_size = len(word_vocab)

    model = CompletionModelV3(config)

    checkpoint = torch.load(CHECKPOINT_PATH, map_location="cpu")
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()

    context_ids = torch.randint(
        0, config.word_vocab_size, (1, CONTEXT_LEN), dtype=torch.long
    )

    candidate_ids = torch.randint(
        0, config.word_vocab_size, (1, CANDIDATE_COUNT), dtype=torch.long
    )

    # 模拟真实场景：候选数量可能不足 batch 内最大值，用 mask 标记有效位置
    # 这里为了简单先全设为 True（真实推理时按需构造）
    candidate_mask = torch.ones((1, CANDIDATE_COUNT), dtype=torch.bool)

    with torch.no_grad():
        logits = model(context_ids, candidate_ids, candidate_mask)

    test_case = {
        "context_ids": context_ids.tolist(),
        "candidate_ids": candidate_ids.tolist(),
        "candidate_mask": candidate_mask.tolist(),
        "expected_logits": logits.tolist(),
    }

    Path(OUTPUT_JSON).parent.mkdir(parents=True, exist_ok=True)

    with open(OUTPUT_JSON, "w", encoding="utf8") as f:
        json.dump(test_case, f)

    print(f"Test case saved to {OUTPUT_JSON}")
    print(f"logits shape: {logits.shape}")


if __name__ == "__main__":
    main()