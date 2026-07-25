import sys
from pathlib import Path

current_file = Path(__file__).resolve()
outer_folder = current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import torch
from torch.utils.data import DataLoader

from structure.completion_model import CompletionModel
from structure.model_config import ModelConfig

from structure.ch_vocabulary import CharacterVocabulary
from structure.word_vocabulary import WordVocabulary

from structure.completion_dataset import CompletionDataset
from structure.collator import collator

from training.trainer import Trainer
from training.training_config import TrainingConfig

import torch.nn as nn


def main():

    # ----------------------------------------------------
    # Config
    # ----------------------------------------------------

    tr_config = TrainingConfig()
    md_config = ModelConfig()

    char_vocab = CharacterVocabulary()

    word_vocab = WordVocabulary(
        "./model/datasets/vocabulary/word2id.json"
    )

    md_config.word_vocab_size = len(word_vocab)
    md_config.char_vocab_size = len(char_vocab)

    device = torch.device(
        "cuda"
        if torch.cuda.is_available()
        else "cpu"
    )

    # ----------------------------------------------------
    # Model
    # ----------------------------------------------------

    model = CompletionModel(md_config)

    checkpoint = torch.load(
        "./model/results/models/best.pt",
        map_location=device
    )

    model.load_state_dict(
        checkpoint["model_state_dict"]
    )

    criterion = nn.CrossEntropyLoss()

    trainer = Trainer(

        model,

        optimizer=None,

        criterion=criterion,

        device=device

    )

    # ----------------------------------------------------
    # Dataset
    # ----------------------------------------------------

    test_dataset = CompletionDataset(

        "./model/datasets/processed/test.jsonl",

        word_vocab,

        char_vocab

    )

    test_loader = DataLoader(

        test_dataset,

        batch_size=tr_config.batch_size,

        shuffle=False,

        collate_fn=collator,

        num_workers=4,

        pin_memory=True

    )

    # ----------------------------------------------------
    # Evaluation
    # ----------------------------------------------------

    print()

    print("=" * 60)
    print("Testing...")
    print("=" * 60)

    result = trainer.evaluate(
        test_loader
    )

    print()

    print("=" * 60)
    print("Test Result")
    print("=" * 60)

    for k, v in result.items():

        if isinstance(v, float):

            print(f"{k:<20}: {v:.6f}")

        else:

            print(f"{k:<20}: {v}")


if __name__ == "__main__":
    main()