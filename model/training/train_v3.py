import sys
from pathlib import Path

current_file=Path(__file__).resolve()

outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:

    sys.path.insert(
        0,
        str(outer_folder)
    )

import logging
import torch
import torch.nn as nn

from torch.optim import AdamW

from torch.utils.data import DataLoader


from structure.completion_model_v3 import CompletionModelV3

from training.trainer_v3 import TrainerV3

from training_config import TrainingConfig

from structure.model_config import ModelConfig


from structure.ch_vocabulary import CharacterVocabulary

from structure.word_vocabulary import WordVocabulary


from structure.collator_v3 import collator_v3

from structure.completion_dataset_v3 import CompletionDatasetV3

from training.metrics_v3 import MetricsV3


# ==========================
# Checkpoint / Log 路径
# ==========================

CHECKPOINT_DIR = Path("model/results/checkpoints_v3")
LATEST_CHECKPOINT_PATH = CHECKPOINT_DIR / "latest.pt"
BEST_CHECKPOINT_PATH = CHECKPOINT_DIR / "best.pt"

LOG_DIR = Path("model/results/logs")
LOG_PATH = LOG_DIR / "train_v3.log"

CHECKPOINT_DIR.mkdir(parents=True, exist_ok=True)
LOG_DIR.mkdir(parents=True, exist_ok=True)

# 判定"最佳"的指标：valid_result 里的 key，loss 越低越好
BEST_METRIC_KEY = "loss"
BEST_METRIC_MODE = "min"  # "min" 或 "max"

SAVE_INTERVAL = 50_000  # 每 5w iteration 保存一次 latest.pt

# ==========================
# Logger 配置：同时输出到文件和终端
# ==========================

logger = logging.getLogger("train_v3")
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


# ==========================
# Config
# ==========================

tr_config = TrainingConfig()

md_config = ModelConfig()


# ==========================
# Vocabulary
# ==========================

char_vocab = CharacterVocabulary()

word_vocab = WordVocabulary(
    "./model/datasets/vocabulary/word2id.json"
)


md_config.word_vocab_size = len(word_vocab)

md_config.char_vocab_size = len(char_vocab)


device = torch.device(

    "cuda"

    if torch.cuda.is_available()

    else "cpu"

)


# ==========================
# Model
# ==========================

model = CompletionModelV3(
    md_config
)


# ==========================
# Optimizer
# ==========================

optimizer = AdamW(

    model.parameters(),

    lr=tr_config.learning_rate,

    weight_decay=1e-2

)


# ==========================
# Metrics
# ==========================

metrics = MetricsV3()


# ==========================
# Dataset
# ==========================

train_dataset = CompletionDatasetV3(

    "./model/datasets/processed/train_v3.jsonl",

    "./model/datasets/candidate/prefix_candidates.json",

    word_vocab,

)


valid_dataset = CompletionDatasetV3(

    "./model/datasets/processed/valid_v3.jsonl",

    "./model/datasets/candidate/prefix_candidates.json",

    word_vocab,

)


valid_dataloader = DataLoader(

    valid_dataset,

    batch_size=tr_config.batch_size,

    shuffle=False,

    collate_fn=collator_v3

)


def make_train_dataloader(epoch):

    """
    shuffle=True 时，用按 epoch 固定的随机种子重建 DataLoader，
    保证同一个 epoch 无论重启多少次，打乱顺序都完全一致，
    这样断点恢复时"跳过前 N 个 batch"才是确定性、可复现的。
    """

    generator = torch.Generator()
    generator.manual_seed(epoch)

    return DataLoader(

        train_dataset,

        batch_size=tr_config.batch_size,

        shuffle=True,

        collate_fn=collator_v3,

        generator=generator

    )


# ==========================
# 断点恢复状态
# ==========================

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


def checkpoint_callback(epoch, global_step, batch_in_epoch, loss):

    save_checkpoint(
        LATEST_CHECKPOINT_PATH,
        epoch=epoch,
        global_step=global_step,
        batch_in_epoch=batch_in_epoch
    )

    logger.info(
        f"[step {global_step}] loss {loss:.4f} | checkpoint saved "
        f"(epoch {epoch + 1}, batch {batch_in_epoch}) "
        f"to {LATEST_CHECKPOINT_PATH}"
    )


trainer = TrainerV3(

    model,

    optimizer,

    device,

    metrics,

    checkpoint_callback=checkpoint_callback,
    save_interval=SAVE_INTERVAL

)


# ==========================
# 加载 checkpoint（如果存在）
# ==========================

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
    if start_skip_batches >= len(train_dataset) // tr_config.batch_size:
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


# ==========================
# Train
# ==========================

for epoch in range(start_epoch, tr_config.num_epochs):

    train_dataloader = make_train_dataloader(epoch)

    skip_batches = start_skip_batches if epoch == start_epoch else 0

    train_loss = trainer.train_epoch(

        train_dataloader,

        epoch=epoch,

        skip_batches=skip_batches

    )

    valid_result = trainer.evaluate(

        valid_dataloader

    )

    logger.info(f"Epoch {epoch + 1}/{tr_config.num_epochs}")

    logger.info(f"Train | loss: {train_loss:.4f}")

    logger.info(
        "Valid | "
        + " | ".join(f"{k}: {v:.4f}" for k, v in valid_result.items())
    )

    # ------------------------
    # epoch 结束：保存 latest（batch_in_epoch = 总长度，
    # 代表本 epoch 已完整跑完，下次恢复自动进入下一 epoch）
    # ------------------------

    current_metric = valid_result[BEST_METRIC_KEY]

    if is_better(current_metric, best_metric):
        best_metric = current_metric

    save_checkpoint(
        LATEST_CHECKPOINT_PATH,
        epoch=epoch,
        global_step=trainer.global_step,
        batch_in_epoch=len(train_dataloader),
        extra={
            "train_loss": train_loss,
            "valid_result": valid_result
        }
    )

    logger.info(f"Epoch checkpoint saved to {LATEST_CHECKPOINT_PATH}")

    # ------------------------
    # 保存最佳 checkpoint
    # ------------------------

    if is_better(valid_result[BEST_METRIC_KEY], best_metric) or valid_result[BEST_METRIC_KEY] == best_metric:

        save_checkpoint(
            BEST_CHECKPOINT_PATH,
            epoch=epoch,
            global_step=trainer.global_step,
            batch_in_epoch=len(train_dataloader),
            extra={
                "train_loss": train_loss,
                "valid_result": valid_result
            }
        )

        logger.info(
            f"New best {BEST_METRIC_KEY}: {best_metric:.4f}, "
            f"saved to {BEST_CHECKPOINT_PATH}"
        )

    logger.info("-" * 60)