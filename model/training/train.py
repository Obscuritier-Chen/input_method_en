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

# iteration 级别保存间隔
SAVE_INTERVAL = 50_000

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
# 断点恢复状态
# -------------------------

start_epoch = 0
start_skip_batches = 0

if BEST_METRIC_MODE == "min":
    best_metric = float("inf")
else:
    best_metric = float("-inf")


def is_better(current, best):

    if BEST_METRIC_MODE == "min":

        return current < best

    else:

        return current > best


def save_checkpoint(path, epoch, global_step, batch_in_epoch, extra=None):

    payload = {
        "epoch": epoch,
        "global_step": global_step,
        "batch_in_epoch": batch_in_epoch,
        "model_state_dict": model.state_dict(),
        "optimizer_state_dict": optimizer.state_dict(),
        "best_metric": best_metric,
    }

    if extra:
        payload.update(extra)

    torch.save(payload, path)


# -------------------------
# iteration 级别保存回调，由 Trainer 在 train_epoch 内部按 save_interval 触发
# -------------------------

def checkpoint_callback(epoch, global_step, batch_in_epoch):

    save_checkpoint(
        LATEST_CHECKPOINT_PATH,
        epoch=epoch,
        global_step=global_step,
        batch_in_epoch=batch_in_epoch
    )

    logger.info(
        f"[step {global_step}] checkpoint saved "
        f"(epoch {epoch + 1}, batch {batch_in_epoch}) "
        f"to {LATEST_CHECKPOINT_PATH}"
    )


trainer = Trainer(

    model,
    optimizer,
    criterion,
    device,
    checkpoint_callback=checkpoint_callback,
    save_interval=SAVE_INTERVAL

)

# -------------------------
# 加载 checkpoint（如果存在）
# -------------------------

if LATEST_CHECKPOINT_PATH.exists():

    logger.info(f"Found checkpoint at {LATEST_CHECKPOINT_PATH}, resuming...")

    checkpoint = torch.load(LATEST_CHECKPOINT_PATH, map_location=device)

    model.load_state_dict(checkpoint["model_state_dict"])

    optimizer.load_state_dict(checkpoint["optimizer_state_dict"])

    trainer.global_step = checkpoint.get("global_step", 0)

    start_epoch = checkpoint["epoch"]

    start_skip_batches = checkpoint.get("batch_in_epoch", 0)

    best_metric = checkpoint.get("best_metric", best_metric)

    # 如果上次保存点已经是本 epoch 的最后一个 batch，直接进入下一个 epoch
    if start_skip_batches >= len(train_dataloader):
        start_epoch += 1
        start_skip_batches = 0

    logger.info(
        f"Resumed at epoch {start_epoch + 1}, "
        f"batch {start_skip_batches}, "
        f"global_step {trainer.global_step}, "
        f"current best {BEST_METRIC_KEY}: {best_metric:.4f}"
    )

else:

    logger.info("No checkpoint found, starting from scratch.")

# -------------------------
# 训练循环
# -------------------------

for epoch in range(start_epoch, tr_config.num_epochs):

    skip_batches = start_skip_batches if epoch == start_epoch else 0

    train_result = trainer.train_epoch(
        train_dataloader,
        epoch=epoch,
        skip_batches=skip_batches
    )

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
    # epoch 结束：保存 latest（batch_in_epoch = 总长度，
    # 代表本 epoch 已完整跑完，下次恢复自动进入下一 epoch）
    # -------------------------

    current_metric = valid_result[BEST_METRIC_KEY]

    if is_better(current_metric, best_metric):
        best_metric = current_metric

    save_checkpoint(
        LATEST_CHECKPOINT_PATH,
        epoch=epoch,
        global_step=trainer.global_step,
        batch_in_epoch=len(train_dataloader),
        extra={
            "train_result": train_result,
            "valid_result": valid_result
        }
    )

    logger.info(f"Epoch checkpoint saved to {LATEST_CHECKPOINT_PATH}")

    # -------------------------
    # 保存最佳 checkpoint（valid 指标更优时才覆盖）
    # -------------------------

    if is_better(current_metric, best_metric) or current_metric == best_metric:

        save_checkpoint(
            BEST_CHECKPOINT_PATH,
            epoch=epoch,
            global_step=trainer.global_step,
            batch_in_epoch=len(train_dataloader),
            extra={
                "train_result": train_result,
                "valid_result": valid_result
            }
        )

        logger.info(
            f"New best {BEST_METRIC_KEY}: {best_metric:.4f}, "
            f"saved to {BEST_CHECKPOINT_PATH}"
        )

    logger.info("-" * 60)