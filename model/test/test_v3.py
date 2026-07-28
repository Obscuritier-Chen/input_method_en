import sys
from pathlib import Path

current_file = Path(__file__).resolve()

outer_folder = current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))


import torch
import torch.nn as nn

from torch.utils.data import DataLoader


from structure.completion_model_v3 import CompletionModelV3

from structure.model_config import ModelConfig
from training.training_config import TrainingConfig


from structure.word_vocabulary import WordVocabulary

from structure.completion_dataset_v3 import CompletionDatasetV3

from structure.collator_v3 import collator_v3

from training.metrics_v3 import MetricsV3

from training.trainer_v3 import TrainerV3

def main():

    # ==================================================
    # Config
    # ==================================================

    tr_config = TrainingConfig()

    md_config = ModelConfig()


    word_vocab = WordVocabulary(
        "./model/datasets/vocabulary/word2id.json"
    )


    md_config.word_vocab_size = len(word_vocab)



    device = torch.device(

        "cuda"

        if torch.cuda.is_available()

        else "cpu"

    )



    # ==================================================
    # Model
    # ==================================================

    model = CompletionModelV3(
        md_config
    )


    checkpoint = torch.load(

        "./model/results/models/best_v3.pt",

        map_location=device

    )


    model.load_state_dict(

        checkpoint["model_state_dict"]

    )


    model.to(device)



    print(
        "Model loaded."
    )



    # ==================================================
    # Dataset
    # ==================================================

    test_dataset = CompletionDatasetV3(

        jsonl_path=
        "./model/datasets/processed/test_v3.jsonl",

        candidate_path=
        "./model/datasets/candidate/prefix_candidates.json",

        word_vocab=word_vocab

    )



    test_loader = DataLoader(

        test_dataset,

        batch_size=tr_config.batch_size,

        shuffle=False,

        collate_fn=collator_v3,

        num_workers=4,

        pin_memory=True

    )



    # ==================================================
    # Evaluation
    # ==================================================

    metrics = MetricsV3()

    trainer = TrainerV3(

        model=model,

        optimizer=None,

        device=device,

        metrics=metrics

    )



    print()

    print("="*60)

    print("Testing V3")

    print("="*60)



    result = trainer.evaluate(

        test_loader

    )



    print()

    print("="*60)

    print("Test Result V3")

    print("="*60)



    for k,v in result.items():

        if isinstance(v,float):

            print(
                f"{k:<20}: {v:.6f}"
            )

        else:

            print(
                f"{k:<20}: {v}"
            )

if __name__ == "__main__":
    main()