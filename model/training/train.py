import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

import logging
import torch
import torch.nn as nn

from torch.optim import AdamW

from torch.utils.data import DataLoader

from structure.completion_model import CompletionModel

from training.trainer import Trainer

from training_config import TrainingConfig
from structure.model_config import ModelConfig

from structure.ch_vocabulary import CharacterVocabulary
from structure.word_vocabulary import WordVocabulary

from structure.collator import collator
from structure.completion_dataset import CompletionDataset

# -------------------------
# Checkpoint / Log 路径
# -------------------------

CHECKPOINT_DIR = Path("model/results/checkpoints")
LATEST_CHECKPOINT_PATH = CHECKPOINT_DIR / "latest.pt"
BEST_CHECKPOINT_PATH = CHECKPOINT_DIR / "best.pt"

LOG_DIR = Path("model/results/logs")
LOG_PATH = LOG_DIR / "train.log"

CHECKPOINT_DIR.mkdir(parents=True, exist_ok=True)
LOG_DIR.mkdir(parents=True, exist_ok=True)

# 判定"最佳"的指标：默认用 valid loss，越低越好
# 如果想用 top1/mrr 等"越高越好"的指标，把 BEST_METRIC_MODE 改成 "max"
BEST_METRIC_KEY = "loss"
BEST_METRIC_MODE = "min"  # "min" 或 "max"

# -------------------------
# Logger 配置：同时输出到文件和终端
# -------------------------

logger = logging.getLogger("train")
logger.setLevel(logging.INFO)

if not logger.handlers:

    file_handler = logging.FileHandler(LOG_PATH, encoding="utf8")
    file_handler.setFormatter(
        logging.Formatter("%(asctime)s [%(levelname)s] %(message)s")
    )

    stream_handler = logging.StreamHandler()
    stream_handler.setFormatter(
        logging.Formatter("%(message)s")
    )

    logger.addHandler(file_handler)
    logger.addHandler(stream_handler)

# -------------------------

tr_config = TrainingConfig()
md_config = ModelConfig()

# 这里填入 Vocabulary 大小
char_vocab=CharacterVocabulary()
word_vocab=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')

md_config.word_vocab_size = len(word_vocab)
md_config.char_vocab_size = len(char_vocab)

device = torch.device(

    "cuda"

    if torch.cuda.is_available()

    else "cpu"

)

model = CompletionModel(md_config)

criterion = nn.CrossEntropyLoss()

optimizer = AdamW(
    model.parameters(),
    lr=tr_config.learning_rate,
    weight_decay=1e-2
)

trainer = Trainer(

    model,
    optimizer,
    criterion,
    device

)

train_dataset=CompletionDataset(r'./model/datasets/processed/train.jsonl',
                            word_vocab,
                            char_vocab)
valid_dataset=CompletionDataset(r'./model/datasets/processed/valid.jsonl',
                            word_vocab,
                            char_vocab)

train_dataloader=DataLoader(
                    train_dataset,
                    batch_size=tr_config.batch_size,
                    shuffle=False,
                    collate_fn=collator
                    )

valid_dataloader=DataLoader(
                    valid_dataset,
                    batch_size=tr_config.batch_size,
                    shuffle=False,
                    collate_fn=collator
                    )

# -------------------------
# 断点恢复
# -------------------------

start_epoch = 0

if BEST_METRIC_MODE == "min":
    best_metric = float("inf")
else:
    best_metric = float("-inf")


def is_better(current, best):

    if BEST_METRIC_MODE == "min":

        return current < best

    else:

        return current > best


if LATEST_CHECKPOINT_PATH.exists():

    logger.info(f"Found checkpoint at {LATEST_CHECKPOINT_PATH}, resuming...")

    checkpoint = torch.load(LATEST_CHECKPOINT_PATH, map_location=device)

    model.load_state_dict(checkpoint["model_state_dict"])

    optimizer.load_state_dict(checkpoint["optimizer_state_dict"])

    start_epoch = checkpoint["epoch"]

    best_metric = checkpoint.get("best_metric", best_metric)

    logger.info(
        f"Resumed from epoch {start_epoch}, "
        f"current best {BEST_METRIC_KEY}: {best_metric:.4f}"
    )

else:

    logger.info("No checkpoint found, starting from scratch.")

# -------------------------
# 训练循环
# -------------------------

for epoch in range(start_epoch, tr_config.num_epochs):

    train_result = trainer.train_epoch(train_dataloader)

    valid_result = trainer.evaluate(valid_dataloader)

    logger.info(f"Epoch {epoch + 1}/{tr_config.num_epochs}")

    logger.info(
        "Train | "
        + " | ".join(f"{k}: {v:.4f}" for k, v in train_result.items())
    )

    logger.info(
        "Valid | "
        + " | ".join(f"{k}: {v:.4f}" for k, v in valid_result.items())
    )

    # -------------------------
    # 保存最新 checkpoint（覆盖上一个 epoch，用于断点恢复）
    # -------------------------

    current_metric = valid_result[BEST_METRIC_KEY]

    checkpoint_payload = {
        "epoch": epoch + 1,
        "model_state_dict": model.state_dict(),
        "optimizer_state_dict": optimizer.state_dict(),
        "train_result": train_result,
        "valid_result": valid_result,
        "best_metric": (
            current_metric
            if is_better(current_metric, best_metric)
            else best_metric
        ),
    }

    torch.save(checkpoint_payload, LATEST_CHECKPOINT_PATH)

    logger.info(f"Checkpoint saved to {LATEST_CHECKPOINT_PATH}")

    # -------------------------
    # 保存最佳 checkpoint（valid 指标更优时才覆盖）
    # -------------------------

    if is_better(current_metric, best_metric):

        best_metric = current_metric

        torch.save(checkpoint_payload, BEST_CHECKPOINT_PATH)

        logger.info(
            f"New best {BEST_METRIC_KEY}: {best_metric:.4f}, "
            f"saved to {BEST_CHECKPOINT_PATH}"
        )

    logger.info("-" * 60)