import torch.nn as nn

from .position_encoding import PositionalEncoding
from .word_embedding import WordEmbedding
from .embedding_builder_v3 import EmbeddingBuilderV3
from .transformer_encoder_v3 import TransformerEncoderV3
from .context_projection import ContextProjection
from .prediction_head_v3 import PredictionHeadV3


class CompletionModelV3(nn.Module):

    def __init__(self,config):

        super().__init__()

        word_embedding=WordEmbedding(config)
        positional_encoding=PositionalEncoding(config)

        self.embedding_builder=EmbeddingBuilderV3(
            word_embedding,
            positional_encoding,
            config,                           
        )

        self.transformer_encoder=TransformerEncoderV3(config)
        self.context_projection = ContextProjection(config)
        self.prediction_head = PredictionHeadV3()

    def forward(
        self,
        context_ids,
        candidate_ids,
        candidate_mask
    ):
        """
        Parameters
        ----------
        context_ids
            [B,L]

        candidate_ids
            [B,K]

        Returns
        -------
        logits
            [B,K]
        """

        ##################################################
        # Embedding
        ##################################################

        embedding = self.embedding_builder(

            context_ids,

            candidate_ids

        )

        context_embedding = embedding[

            "context_embedding"

        ]

        candidate_embedding = embedding[

            "candidate_embedding"

        ]

        ##################################################
        # Transformer
        ##################################################

        context_vector = self.transformer_encoder(

            context_embedding,

            context_ids

        )

        ##################################################
        # Projection
        ##################################################

        context_vector = self.context_projection(

            context_vector

        )

        ##################################################
        # Prediction
        ##################################################

        logits = self.prediction_head(

            context_vector,

            candidate_embedding,

            candidate_mask

        )

        ##################################################

        return logits