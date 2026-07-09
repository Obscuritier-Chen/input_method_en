import torch
import torch.nn as nn


class PrefixEncoder(nn.Module):

    def __init__(
        self,
        vocab_size,
        config,
        padding_idx=0
    ):

        super().__init__()

        self.embedding = nn.Embedding(

            vocab_size,

            config.embedding_dim,

            padding_idx=padding_idx

        )

    def forward(self, prefix_ids):

        x = self.embedding(prefix_ids)

        mask = (prefix_ids != 0).unsqueeze(-1)

        x = x * mask

        length = mask.sum(dim=1).clamp(min=1)

        x = x.sum(dim=1) / length

        return x