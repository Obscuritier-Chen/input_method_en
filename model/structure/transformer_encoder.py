import torch
import torch.nn as nn


class TransformerEncoder(nn.Module):

    def __init__(
        self,
        config,
        ):

        super().__init__()

        encoder_layer = nn.TransformerEncoderLayer(

            d_model=config.embedding_dim,

            nhead=config.num_heads,

            dim_feedforward=config.hidden_dim,

            dropout=config.dropout,

            activation="gelu",

            batch_first=True,

            norm_first=True
        )

        self.encoder = nn.TransformerEncoder(

            encoder_layer,

            num_layers=config.num_layers
        )

    def forward(
        self,
        embeddings,
        padding_mask
    ):

        return self.encoder(

            src=embeddings,

            src_key_padding_mask=~padding_mask
        )