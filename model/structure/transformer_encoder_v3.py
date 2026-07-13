import torch
import torch.nn as nn


class TransformerEncoder(nn.Module):

    def __init__(
        self,
        embedding_dim,
        num_heads,
        hidden_dim,
        num_layers,
        dropout=0.1
    ):

        super().__init__()

        encoder_layer = nn.TransformerEncoderLayer(

            d_model=embedding_dim,

            nhead=num_heads,

            dim_feedforward=hidden_dim,

            dropout=dropout,

            activation="gelu",

            batch_first=True

        )

        self.encoder = nn.TransformerEncoder(

            encoder_layer,

            num_layers=num_layers

        )

    def forward(
        self,
        context_embedding,
        context_ids,
        padding_idx=0
    ):
        """
        Parameters
        ----------
        context_embedding
            [B,L+1,H]

        context_ids
            [B,L]
        """

        ##################################################
        # Padding Mask
        ##################################################

        padding_mask = (

            context_ids == padding_idx

        )

        ##################################################
        # CLS 永远不是 Padding
        ##################################################

        cls_mask = torch.zeros(

            padding_mask.size(0),

            1,

            dtype=torch.bool,

            device=padding_mask.device

        )

        padding_mask = torch.cat(

            [

                cls_mask,

                padding_mask

            ],

            dim=1

        )

        ##################################################

        output = self.encoder(

            context_embedding,

            src_key_padding_mask=padding_mask

        )

        ##################################################
        # CLS
        ##################################################

        cls = output[:, 0]

        return cls