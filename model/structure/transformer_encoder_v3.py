import torch
import torch.nn as nn


class TransformerEncoderV3(nn.Module):

    def __init__(
        self,
        config
    ):

        super().__init__()

        encoder_layer = nn.TransformerEncoderLayer(

            d_model=config.embedding_dim,

            nhead=config.num_heads,

            dim_feedforward=config.hidden_dim,

            dropout=config.dropout,

            activation="gelu",

            batch_first=True

        )

        self.encoder = nn.TransformerEncoder(

            encoder_layer,

            num_layers=config.num_layers

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