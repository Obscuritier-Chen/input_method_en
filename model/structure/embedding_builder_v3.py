import torch
import torch.nn as nn


class EmbeddingBuilderV3(nn.Module):

    def __init__(
        self,
        word_embedding,
        position_embedding,
        config
    ):

        super().__init__()

        self.word_embedding = word_embedding

        self.position_embedding = position_embedding

        ##################################################
        # Learnable CLS Token
        ##################################################

        self.cls_embedding = nn.Parameter(

            torch.randn(
                1,
                1,
                config.embedding_dim
            )

        )

    def forward(
        self,
        context_ids,
        candidate_ids
    ):
        """
        Parameters
        ----------
        context_ids
            [B, L]

        candidate_ids
            [B, K]

        Returns
        -------
        context_embedding
            [B, L+1, H]

        candidate_embedding
            [B, K, H]
        """

        ##################################################
        # Context Word Embedding
        ##################################################

        context_embedding = self.word_embedding(
            context_ids
        )

        ##################################################
        # Candidate Word Embedding
        ##################################################

        candidate_embedding = self.word_embedding(
            candidate_ids
        )

        ##################################################
        # Prepend Learnable CLS
        ##################################################

        batch_size = context_embedding.size(0)

        cls_embedding = self.cls_embedding.expand(
            batch_size,
            -1,
            -1
        )

        context_embedding = torch.cat(
            [
                cls_embedding,
                context_embedding
            ],
            dim=1
        )

        ##################################################
        # Position Embedding
        #
        # 不修改 PositionEmbedding 接口，因此构造
        # 一个假的 context_ids，仅用于生成
        # 长度为 L+1 的位置编码。
        ##################################################

        context_embedding = self.position_embedding(
            context_embedding
        )

        ##################################################

        return {

            "context_embedding": context_embedding,

            "candidate_embedding": candidate_embedding

        }