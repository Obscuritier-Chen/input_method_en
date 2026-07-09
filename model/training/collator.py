import torch
from torch.nn.utils.rnn import pad_sequence

from torch.utils.data import DataLoader

from completion_dataset import CompletionDataset
from ch_vocabulary import CharacterVocabulary
from word_vocabulary import WordVocabulary

def collator(batch):

    context_ids = pad_sequence(
        [sample["context_ids"] for sample in batch],
        batch_first=True,
        padding_value=0
    )

    prefix_ids = pad_sequence(
        [sample["prefix_ids"] for sample in batch],
        batch_first=True,
        padding_value=0
    )

    labels = torch.stack(
        [sample["label"] for sample in batch]
    )

    context_mask = (context_ids != 0)

    prefix_mask = (prefix_ids != 0)

    return {

        "context_ids": context_ids,

        "context_mask": context_mask,

        "prefix_ids": prefix_ids,

        "prefix_mask": prefix_mask,

        "labels": labels
    }

if __name__=='__main__':
    word_voc=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')
    ch_voc=CharacterVocabulary()

    dataset=CompletionDataset(r'./model/datasets/processed/test.jsonl',
                              word_voc,
                              ch_voc)
    
    loader=DataLoader(
        dataset,
        batch_size=4,
        shuffle=False,
        collate_fn=collator
    )

    batch=next(iter(loader))

    print(batch['context_ids'])
    print(batch['prefix_ids'])
    print(batch['labels'])