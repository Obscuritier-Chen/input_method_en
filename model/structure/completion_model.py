import torch
import torch.nn as nn

from .embedding_builder import EmbeddingBuilder
from .word_embedding import WordEmbedding
from .position_encoding import PositionalEncoding
from .prefix_encode import PrefixEncoder
from .type_embedding import TypeEmbedding
from .transformer_encoder import TransformerEncoder
from .prediction_head import PredictionHead


class CompletionModel(nn.Module):

    def __init__(self, config):

        super().__init__()

        # -------------------------
        # Embedding Modules
        # -------------------------

        word_embedding = WordEmbedding(config)

        prefix_encoder = PrefixEncoder(config)

        positional_encoding = PositionalEncoding(config)

        type_embedding = TypeEmbedding(config)

        # -------------------------
        # Embedding Builder
        # -------------------------

        self.embedding_builder = EmbeddingBuilder(
            word_embedding=word_embedding,
            prefix_encoder=prefix_encoder,
            positional_encoding=positional_encoding,
            type_embedding=type_embedding
        )

        # -------------------------
        # Transformer
        # -------------------------

        self.encoder = TransformerEncoder(config)

        # -------------------------
        # Prediction Head
        # -------------------------

        self.prediction_head = PredictionHead(
            embedding_dim=config.embedding_dim,
            vocab_size=config.word_vocab_size
        )

    def forward(
        self,
        context_ids,
        prefix_ids,
        context_mask
    ):

        embeddings, padding_mask = self.embedding_builder(
            context_ids=context_ids,
            prefix_ids=prefix_ids,
            context_mask=context_mask
        )

        encoder_output = self.encoder(
            embeddings=embeddings,
            padding_mask=padding_mask
        )

        logits = self.prediction_head(
            encoder_output
        )

        return logits