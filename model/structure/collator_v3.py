import torch

from torch.nn.utils.rnn import pad_sequence


class CollatorV3:

    def __init__(
        self,
        padding_idx=0
    ):

        self.padding_idx = padding_idx

    def __call__(self, batch):

        ##################################################
        # Context
        ##################################################

        context_ids = [

            sample["context_ids"]

            for sample in batch

        ]

        context_ids = pad_sequence(

            context_ids,

            batch_first=True,

            padding_value=self.padding_idx

        )

        ##################################################
        # Candidate
        ##################################################

        candidate_ids_list = [
            sample["candidate_ids"]
            for sample in batch
        ]

        assert all(
            x.size(0) == candidate_ids_list[0].size(0)
            for x in candidate_ids_list
        ), "Candidate length mismatch."

        candidate_ids = torch.stack(
            candidate_ids_list,
            dim=0
        )

        ##################################################
        # Label
        ##################################################

        labels = torch.stack(

            [

                sample["label"]

                for sample in batch

            ],

            dim=0

        )

        ##################################################

        return {

            "context_ids": context_ids,

            "candidate_ids": candidate_ids,

            "labels": labels

        }