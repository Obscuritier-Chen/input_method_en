import json

import torch

from torch.utils.data import Dataset

PREFIX_CANDIDATES_PATH=r'model/datasets/candidate/preix_candidates.json'

class CompletionDatasetV3(Dataset):

    def __init__(
        self,
        jsonl_path,
        word_vocab
    ):
        """
        Parameters
        ----------
        jsonl_path : str
            train_v3.jsonl / valid_v3.jsonl / test_v3.jsonl

        word_vocab : WordVocabulary
        """

        self.offsets = []

        with open(
            jsonl_path,
            "r",
            encoding="utf8"
        ) as f:

            offset = 0

            print("Building index...")

            for line in f:

                self.offsets.append(offset)

                offset += len(line)

        print(
            f"Loaded {len(self.offsets)} samples."
        )        
        
        with open(PREFIX_CANDIDATES_PATH) as f:
            self.candidate_table=json.load(f)

        self.jsonl_path = jsonl_path

        self.word_vocab = word_vocab

    def __len__(self):

        return len(self.offsets)

    def __getitem__(self, idx):

        with open(
            self.jsonl_path,
            "r",
            encoding="utf8"
        ) as f:

            f.seek(
                self.offsets[idx]
            )

            sample = json.loads(
                f.readline()
            )

        ##################################################
        # Context
        ##################################################

        context_ids = self.candidate_table[
            sample["prefix"]
        ]

        ##################################################
        # Candidate
        ##################################################

        candidate_ids = sample["candidate_ids"]

        ##################################################
        # Label
        ##################################################

        label = sample["label"]

        ##################################################

        return {

            "context_ids":

                torch.tensor(

                    context_ids,

                    dtype=torch.long

                ),

            "candidate_ids":

                torch.tensor(

                    candidate_ids,

                    dtype=torch.long

                ),

            "label":

                torch.tensor(

                    label,

                    dtype=torch.long

                )

        }