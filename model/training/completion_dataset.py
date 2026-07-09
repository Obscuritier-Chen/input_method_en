import json

import torch
from torch.utils.data import Dataset

from ch_vocabulary import CharacterVocabulary
from word_vocabulary import WordVocabulary

import os
from pathlib import Path

class CompletionDataset(Dataset):

    def __init__(
        self,
        jsonl_path,
        word_vocab,
        char_vocab
    ):
        """
        Parameters
        ----------
        jsonl_path : str
            train.jsonl / valid.jsonl / test.jsonl

        word_vocab : WordVocabulary

        char_vocab : CharacterVocabulary
        """

        """一次性读入会内存溢出
        self.samples = []

        with open(jsonl_path, "r", encoding="utf-8") as f:

            for line in f:

                self.samples.append(
                    json.loads(line)
                )
        """
        self.offsets = [] #建立磁盘字节偏移量索引
        with open(jsonl_path, "rb") as f:
            offset=0

            print('index building...')
            cnt=0
            for line in f:
                cnt+=1
                if cnt%9000000==0:
                    print(f'current progress: {round(cnt/79036164*100,2)}%')
                self.offsets.append(offset)
                offset+=len(line)
            print(f'current progress: 100%')
        
        self.word_vocab = word_vocab
        self.char_vocab = char_vocab
        self.jsonl_path = jsonl_path

    def __len__(self):

        return len(self.offsets)

    def __getitem__(self, idx):

        offset=self.offsets[idx]

        with open(self.jsonl_path, 'r', encoding='utf-8') as f:
            f.seek(offset)
            line=f.readline()
            sample=json.loads(line)


        context_ids = self.word_vocab.encode(
            sample["left"]
        )

        # prefix
        prefix_ids = self.char_vocab.encode(
            sample["prefix"]
        )

        # label
        label = self.word_vocab.word_to_id(
            sample["target"]
        )

        return {

            "context_ids":
                torch.tensor(
                    context_ids,
                    dtype=torch.long
                ),

            "prefix_ids":
                torch.tensor(
                    prefix_ids,
                    dtype=torch.long
                ),

            "label":
                torch.tensor(
                    label,
                    dtype=torch.long
                )
        }

if __name__=='__main__':
    word_voc=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')
    ch_voc=CharacterVocabulary()
    
    dataset=CompletionDataset(r'./model/datasets/processed/train.jsonl',
                              word_voc,
                              ch_voc)
    print(len(dataset))

    print(dataset[0])