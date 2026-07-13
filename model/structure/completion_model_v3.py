import torch.nn as nn


class CompletionModelV3(nn.Module):

    def __init__(
        self,
        embedding_builder,
        transformer_encoder,
        context_projection,
        prediction_head
    ):

        super().__init__()

        self.embedding_builder = embedding_builder

        self.transformer_encoder = transformer_encoder

        self.context_projection = context_projection

        self.prediction_head = prediction_head

    def forward(
        self,
        context_ids,
        candidate_ids
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

            candidate_embedding

        )

        ##################################################

        return logits