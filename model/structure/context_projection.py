import torch.nn as nn


class ContextProjection(nn.Module):

    def __init__(
        self,
        config,
    ):
        super().__init__()

        embedding_dim=config.embedding_dim
        dropout=config.dropout

        self.projection = nn.Sequential(

            nn.Linear(
                embedding_dim,
                embedding_dim
            ),

            nn.GELU(),

            nn.Dropout(
                dropout
            ),

            nn.LayerNorm(
                embedding_dim
            )

        )

    def forward(
        self,
        context_vector
    ):
        """
        Parameters
        ----------
        context_vector
            [B,H]

        Returns
        -------
        projected_context
            [B,H]
        """

        return self.projection(
            context_vector
        )