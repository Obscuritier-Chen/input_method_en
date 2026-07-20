import json
import torch

from torch.utils.data import Dataset


class CompletionDatasetV3(Dataset):

    def __init__(
        self,
        jsonl_path,
        candidate_path,
        word_vocab
    ):
        """
        Parameters
        ----------
        jsonl_path : str
            train_v3.jsonl / valid_v3.jsonl / test_v3.jsonl

        candidate_path : str
            prefix_candidates.json

        word_vocab : WordVocabulary
        """

        ##################################################
        # Build offset index
        ##################################################

        self.offsets = []

        with open(
            jsonl_path,
            "rb",
        ) as f:

            offset = 0

            print("Building sample index...")

            for line in f:

                self.offsets.append(offset)

                offset += len(line)

        print(
            f"Total Samples: {len(self.offsets)}"
        )

        ##################################################
        # Load candidate table
        ##################################################

        print("Loading candidate table...")

        with open(
            candidate_path,
            "rb",
        ) as f:

            self.candidate_table = json.load(f)

        print(
            f"Total Prefixes: {len(self.candidate_table)}"
        )

        ##################################################

        self.word_vocab = word_vocab

        self.jsonl_path = jsonl_path

        self._file = None

    ##################################################

    def _get_file(self):

        """
        每个 DataLoader Worker
        只打开一次文件
        """

        if self._file is None:

            self._file = open(
                self.jsonl_path,
                "rb"
            )

        return self._file

    ##################################################

    def __len__(self):

        return len(self.offsets)

    ##################################################

    def __getitem__(self, idx):

        f = self._get_file()

        f.seek(
            self.offsets[idx]
        )

        sample = json.loads(
            f.readline()
        )

        ##################################################
        # Context
        ##################################################

        context_ids = self.word_vocab.encode(
            sample["left"]
        )

        ##################################################
        # Candidate
        ##################################################

        candidate_ids = self.candidate_table[
            sample["prefix"]
        ]

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