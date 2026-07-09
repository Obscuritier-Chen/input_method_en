import math
import torch
import torch.nn as nn


class PositionalEncoding(nn.Module):

    def __init__(
        self,
        config,
        max_length=512
    ):
        super().__init__()

        embedding_dim=config.embedding_dim

        pe = torch.zeros(max_length, embedding_dim)

        position = torch.arange(
            0,
            max_length,
            dtype=torch.float
        ).unsqueeze(1)

        div_term = torch.exp(
            torch.arange(
                0,
                embedding_dim,
                2
            ).float()
            *
            (-math.log(10000.0) / embedding_dim)
        )

        pe[:, 0::2] = torch.sin(position * div_term)
        pe[:, 1::2] = torch.cos(position * div_term)

        pe = pe.unsqueeze(0)

        self.register_buffer("pe", pe)

    def forward(self, x):

        """
        x : (Batch, Length, Embedding)
        """

        length = x.size(1)

        return x + self.pe[:, :length]