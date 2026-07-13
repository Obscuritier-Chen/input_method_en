import torch
import torch.nn as nn

class PredictionHeadV3(nn.Module):

    def __init__(self):
        super().__init__()

    def forward(
        self,
        context_vector,
        candidate_embedding
    ):
        """
        Parameters
        ----------
        context_vector
            [B,H]

        candidate_embedding
            [B,K,H]

        Returns
        -------
        logits
            [B,K]
        """

        logits = torch.bmm(

            candidate_embedding,

            context_vector.unsqueeze(-1)

        ).squeeze(-1)

        return logits