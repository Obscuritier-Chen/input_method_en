import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import torch
import torch.nn as nn

from torch.optim import AdamW

from torch.utils.data import DataLoader

from structure.completion_model import CompletionModel

from training.trainer import Trainer

from training_config import TrainingConfig
from structure.model_config import ModelConfig

from structure.ch_vocabulary import CharacterVocabulary
from structure.word_vocabulary import WordVocabulary

from structure.collator import collator
from structure.completion_dataset import CompletionDataset

# -------------------------

tr_config = TrainingConfig()
md_config = ModelConfig()

# 这里填入 Vocabulary 大小
char_vocab=CharacterVocabulary()
word_vocab=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')

md_config.word_vocab_size = len(word_vocab)
md_config.char_vocab_size = len(char_vocab)

device = torch.device(

    "cuda"

    if torch.cuda.is_available()

    else "cpu"

)

model = CompletionModel(md_config)

criterion = nn.CrossEntropyLoss()

optimizer = AdamW(
    model.parameters(),
    lr=tr_config.learning_rate,
    weight_decay=1e-2
)

trainer = Trainer(

    model,
    optimizer,
    criterion,
    device

)

train_dataset=CompletionDataset(r'./model/datasets/processed/train.jsonl',
                            word_vocab,
                            char_vocab)
valid_dataset=CompletionDataset(r'./model/datasets/processed/valid.jsonl',
                            word_vocab,
                            char_vocab)

train_dataloader=DataLoader(
                    train_dataset,
                    batch_size=tr_config.batch_size,
                    shuffle=False,
                    collate_fn=collator
                    )

valid_dataloader=DataLoader(
                    valid_dataset,
                    batch_size=tr_config.batch_size,
                    shuffle=False,
                    collate_fn=collator
                    )
            

for epoch in range(tr_config.num_epochs):

    train_result = trainer.train_epoch(train_dataloader)

    valid_result = trainer.evaluate(valid_dataloader)

    print(f"Epoch {epoch+1}")

    print(train_result)

    print(valid_result)