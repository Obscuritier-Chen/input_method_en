import torch
import torch.nn as nn


class PredictionHeadV3(nn.Module):

    def __init__(self):
        super().__init__()

    def forward(
        self,
        context_vector,
        candidate_embedding,
        candidate_mask=None
    ):
        """
        Parameters
        ----------
        context_vector
            [B, H]

        candidate_embedding
            [B, K, H]

        candidate_mask
            [B, K]
            True: 有效候选
            False: Padding
        """

        logits = torch.bmm(
            candidate_embedding,
            context_vector.unsqueeze(-1)
        ).squeeze(-1)

        if candidate_mask is not None:
            logits = logits.masked_fill(
                ~candidate_mask,
                float("-inf")
            )

        return logits