import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

from torch.utils.data import DataLoader

from embedding_builder import EmbeddingBuilder
from completion_dataset import CompletionDataset
from ch_vocabulary import CharacterVocabulary
from word_vocabulary import WordVocabulary
from collator import collator
from word_embedding import WordEmbedding
from position_encoding import PositionalEncoding
from prefix_encode import PrefixEncoder
from type_embedding import TypeEmbedding

from model_config import ModelConfig

if __name__=='__main__':
    config=ModelConfig()

    word_voc=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')
    ch_voc=CharacterVocabulary()

    dataset=CompletionDataset(r'./model/datasets/processed/test.jsonl',
                              word_voc,
                              ch_voc)
    
    loader=DataLoader(
        dataset,
        batch_size=4,
        shuffle=False,
        collate_fn=collator
    )

    batch=next(iter(loader))

    word_embedding=WordEmbedding(config)
    prefix_encoder=PrefixEncoder(config)
    positional_encoding=PositionalEncoding(config)
    type_embedding=TypeEmbedding(config)

    embedding_builder=EmbeddingBuilder(
        word_embedding,
        prefix_encoder,
        positional_encoding,
        type_embedding
    )

    embeddings, mask=embedding_builder(
        batch["context_ids"],
        batch['prefix_ids'],
        batch['context_mask']
    )

    print(embeddings.shape)
    print(mask.shape)