import torch
import torch.nn as nn

class PredictionHead(nn.Module):#实际上就是个mlp

    def __init__(
        self,
        embedding_dim,
        vocab_size
    ):

        super().__init__()

        self.norm = nn.LayerNorm(
            embedding_dim
        )

        self.classifier = nn.Linear(
            embedding_dim,
            vocab_size
        )

    def forward(
        self,
        transformer_output
    ):

        prefix_output = transformer_output[:, 0]

        prefix_output = self.norm(
            prefix_output
        )

        logits = self.classifier(
            prefix_output
        )

        return logits