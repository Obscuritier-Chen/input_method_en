import torch
import torch.nn as nn

class EmbeddingBuilder(nn.Module):

    def __init__(
        self,
        word_embedding,
        prefix_encoder,
        positional_encoding,
        type_embedding
    ):

        super().__init__()

        self.word_embedding = word_embedding
        self.prefix_encoder = prefix_encoder
        self.positional_encoding = positional_encoding
        self.type_embedding = type_embedding

    def forward(
        self,
        context_ids,
        prefix_ids,
        context_mask
    ):
        """
        Parameters
        ----------
        context_ids : (B, L)
            Word ids.

        prefix_ids : (B, P)
            Character ids.

        context_mask : (B, L)
            True -> valid token
            False -> padding

        Returns
        -------
        embeddings : (B, L+1, D)

        padding_mask : (B, L+1)
            True -> valid token
            False -> padding
        """

        # ----------------------------
        # Context Embedding
        # (B,L,D)
        # ----------------------------

        context_embedding = self.word_embedding(
            context_ids
        )

        # ----------------------------
        # Prefix Embedding
        # (B,D)
        # ----------------------------

        prefix_embedding = self.prefix_encoder(
            prefix_ids
        )

        # (B,1,D)

        prefix_embedding = prefix_embedding.unsqueeze(1)

        # ----------------------------
        # Concatenate
        # (B,L+1,D)
        # ----------------------------

        embeddings = torch.cat(
            [
                prefix_embedding,
                context_embedding
            ],
            dim=1
        )

        # ----------------------------
        # Position Embedding
        # ----------------------------

        embeddings = self.positional_encoding(
            embeddings
        )

        # ----------------------------
        # Type Embedding
        # ----------------------------

        embeddings = embeddings + self.type_embedding(
            context_ids
        )

        # ----------------------------
        # Build Padding Mask
        # ----------------------------

        prefix_mask = torch.ones(
            context_mask.size(0),
            1,
            dtype=torch.bool,
            device=context_mask.device
        )

        padding_mask = torch.cat(
            [
                prefix_mask,
                context_mask
            ],
            dim=1
        )

        return embeddings, padding_mask