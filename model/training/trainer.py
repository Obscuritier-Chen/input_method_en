import torch
from tqdm import tqdm
from torch.nn.utils import clip_grad_norm_
import time

from metrics import Metrics


class Trainer:

    def __init__(
        self,
        model,
        optimizer,
        criterion,
        device,
        checkpoint_callback=None,   # 新增：每 save_interval 个 step 调用一次
        save_interval=50_000        # 新增：iteration 级别保存间隔
    ):

        self.model = model.to(device)
        self.optimizer = optimizer
        self.criterion = criterion
        self.device = device

        self.checkpoint_callback = checkpoint_callback
        self.save_interval = save_interval

        self.global_step = 0  # 跨 epoch 累计，断点恢复时从外部赋值恢复

    def train_epoch(
        self,
        dataloader,
        epoch=0,
        skip_batches=0
    ):

        self.model.train()

        metrics = Metrics()

        progress = tqdm(
            dataloader,
            desc="Training",
            leave=False,
            initial=skip_batches,
            total=len(dataloader)
        )

        for batch_idx, batch in enumerate(progress):

            if batch_idx < skip_batches:
                continue

            context_ids = batch["context_ids"].to(self.device)

            prefix_ids = batch["prefix_ids"].to(self.device)

            context_mask = batch["context_mask"].to(self.device)

            labels = batch["label"].to(self.device)

            self.optimizer.zero_grad()

            start_time = time.time()

            logits = self.model(
                context_ids=context_ids,
                prefix_ids=prefix_ids,
                context_mask=context_mask
            )

            loss = self.criterion(
                logits,
                labels
            )

            loss.backward()

            clip_grad_norm_(self.model.parameters(), max_norm=1.0)

            self.optimizer.step()

            elapsed_time = time.time() - start_time

            metrics.update_prediction(
                logits=logits,
                labels=labels,
                loss=loss
            )

            metrics.update_latency(elapsed_time)

            self.global_step += 1

            # ------------------------
            # iteration 级别保存
            # ------------------------

            if (
                self.checkpoint_callback is not None
                and self.global_step % self.save_interval == 0
            ):

                self.checkpoint_callback(
                    epoch=epoch,
                    global_step=self.global_step,
                    batch_in_epoch=batch_idx + 1
                )

            progress.set_postfix(
                loss=loss.item()
            )

        return metrics.compute()

    @torch.no_grad()
    def evaluate(
        self,
        dataloader
    ):

        self.model.eval()

        metrics = Metrics()

        progress = tqdm(
            dataloader,
            desc="Validation",
            leave=False
        )

        for batch in progress:

            context_ids = batch["context_ids"].to(self.device)

            prefix_ids = batch["prefix_ids"].to(self.device)

            context_mask = batch["context_mask"].to(self.device)

            labels = batch["label"].to(self.device)

            start_time = time.time()

            logits = self.model(
                context_ids=context_ids,
                prefix_ids=prefix_ids,
                context_mask=context_mask
            )

            loss = self.criterion(
                logits,
                labels
            )

            elapsed_time = time.time() - start_time

            metrics.update_prediction(
                logits=logits,
                labels=labels,
                loss=loss
            )

            metrics.update_latency(elapsed_time)

        return metrics.compute()