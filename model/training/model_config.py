class ModelConfig:

    def __init__(self):#超参数管理
        self.embedding_dim = 256
        self.num_heads = 8
        self.num_layers = 6
        self.ff_dim = 1024
        self.dropout = 0.1
        self.max_context_length = 64