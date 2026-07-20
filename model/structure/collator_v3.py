import torch
from torch.nn.utils.rnn import pad_sequence


PAD_ID = 0


def collator_v3(batch):

    ##################################################
    # context
    ##################################################

    context_ids = pad_sequence(
        [item["context_ids"] for item in batch],
        batch_first=True,
        padding_value=PAD_ID
    )

    ##################################################
    # candidate
    ##################################################

    candidate_ids = pad_sequence(
        [item["candidate_ids"] for item in batch],
        batch_first=True,
        padding_value=PAD_ID
    )

    ##################################################
    # candidate mask
    ##################################################

    candidate_mask = (candidate_ids != PAD_ID)

    ##################################################
    # label
    ##################################################

    labels = torch.stack(
        [item["label"] for item in batch]
    )

    ##################################################

    return {

        "context_ids": context_ids,

        "candidate_ids": candidate_ids,

        "candidate_mask": candidate_mask,

        "label": labels

    }