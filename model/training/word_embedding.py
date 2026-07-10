import torch
import torch.nn as nn


class WordEmbedding(nn.Module):

    def __init__(
        self,
        config,
        padding_idx=0,
    ):

        super().__init__()
        
        self.embedding = nn.Embedding(
            num_embeddings=config.word_vocab_size,
            embedding_dim=config.embedding_dim,
            padding_idx=padding_idx
        )

    def forward(self, context_ids):

        return self.embedding(context_ids)