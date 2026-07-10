import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

from torch.utils.data import DataLoader

from ch_vocabulary import CharacterVocabulary
from word_vocabulary import WordVocabulary
from collator import collator
from completion_dataset import CompletionDataset

from completion_model import CompletionModel

from model_config import ModelConfig

if __name__=='__main__':
    config=ModelConfig()

    word_voc=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')
    ch_voc=CharacterVocabulary()

    dataset=CompletionDataset(r'./model/datasets/processed/test.jsonl',
                              word_voc,
                              ch_voc)
    
    loader=DataLoader(
        dataset,
        batch_size=config.batch_size,
        shuffle=False,
        collate_fn=collator
    )

    batch=next(iter(loader))

    model=CompletionModel(config)

    logits=model(
        context_ids=batch["context_ids"],
        prefix_ids=batch["prefix_ids"],
        context_mask=batch["context_mask"]
    )

    print(logits.shape)
    print(batch["label"].shape)