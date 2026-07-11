import torch
import torch.nn as nn


class TypeEmbedding(nn.Module):
    """
    Token Type:
        0 -> Prefix
        1 -> Context
    """
    def __init__(self, config):
        super().__init__()
        self.embedding = nn.Embedding(
            num_embeddings=2,
            embedding_dim=config.embedding_dim
        )

    def forward(self, context_ids: torch.Tensor):
        """
        🎯 优化后的 forward 接口
        Parameters
        ----------
        context_ids : torch.Tensor
            形状为 [batch_size, context_length] 的上下文单词张量
        """
        batch_size, context_length = context_ids.shape
        
        # 🎯 工业级避坑：使用 context_ids.new_ones 自动继承其所在的 GPU(device) 和后端
        type_ids = context_ids.new_ones(
            batch_size,
            context_length + 1,  # 留出 1 个位置给 Prefix
            dtype=torch.long
        )

        # 第一个 Token 为 Prefix 标记为 0
        type_ids[:, 0] = 0

        return self.embedding(type_ids)
    
if __name__=='__main__':
    # 模拟从 Dataloader 出来的真实 context_ids (假设在 GPU 上)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    mock_context = torch.randint(10, 1000, (4, 32), device=device) # Batch=4, Len=32
    
    # 实例化
    type_embed_layer = TypeEmbedding(embedding_dim=256).to(device)
    
    # 运算
    out_vectors = type_embed_layer(mock_context)
    
    print("=== 检查结果 ===")
    print(f"输入上下文形状: {mock_context.shape}")
    print(f"输出类型嵌入形状 (应为 [4, 33, 256]): {out_vectors.shape}")
    print(f"数据所在设备验证: {out_vectors.device}")