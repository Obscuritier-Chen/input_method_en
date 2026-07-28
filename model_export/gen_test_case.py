# model_export/gen_test_case.py

import sys
from pathlib import Path

current_file = Path(__file__).resolve()
outer_folder = current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import json
import torch

from model.structure.completion_model import CompletionModel
from model.structure.model_config import ModelConfig
from model.structure.ch_vocabulary import CharacterVocabulary
from model.structure.word_vocabulary import WordVocabulary


CHECKPOINT_PATH = "./model/results/models/best.pt"
VOCAB_PATH = "./model/datasets/vocabulary/word2id.json"
OUTPUT_JSON = "./model_export/output/test_case.json"

CONTEXT_LEN = 8
PREFIX_LEN = 4


def main():

    torch.manual_seed(42)  # 固定随机种子，保证可复现

    char_vocab = CharacterVocabulary()
    word_vocab = WordVocabulary(VOCAB_PATH)

    config = ModelConfig()
    config.word_vocab_size = len(word_vocab)
    config.char_vocab_size = len(char_vocab)

    model = CompletionModel(config)

    checkpoint = torch.load(CHECKPOINT_PATH, map_location="cpu")
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()

    context_ids = torch.randint(0, config.word_vocab_size, (1, CONTEXT_LEN), dtype=torch.long)
    prefix_ids = torch.randint(0, config.char_vocab_size, (1, PREFIX_LEN), dtype=torch.long)
    context_mask = torch.ones((1, CONTEXT_LEN), dtype=torch.bool)

    with torch.no_grad():
        logits = model(context_ids, prefix_ids, context_mask)

    test_case = {
        "context_ids": context_ids.tolist(),
        "prefix_ids": prefix_ids.tolist(),
        "context_mask": context_mask.tolist(),
        "expected_logits": logits.tolist(),
    }

    Path(OUTPUT_JSON).parent.mkdir(parents=True, exist_ok=True)

    with open(OUTPUT_JSON, "w", encoding="utf8") as f:
        json.dump(test_case, f)

    print(f"Test case saved to {OUTPUT_JSON}")
    print(f"logits shape: {logits.shape}")


if __name__ == "__main__":
    main()