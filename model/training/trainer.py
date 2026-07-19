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
        checkpoint_callback=None,
        save_interval=50_000
    ):

        self.model = model.to(device)
        self.optimizer = optimizer
        self.criterion = criterion
        self.device = device

        self.checkpoint_callback = checkpoint_callback
        self.save_interval = save_interval

        self.global_step = 0

    def train_epoch(
        self,
        dataloader,
        epoch=0,
        skip_batches=0
    ):

        self.model.train()

        metrics = Metrics()

        total_batches = len(dataloader)

        data_iter = iter(dataloader)

        # ------------------------
        # 手动逐个跳过，每步都推进 tqdm 显示，避免假死或双重跳过
        # ------------------------

        if skip_batches > 0:

            skip_progress = tqdm(
                range(skip_batches),
                desc="Skipping (resume)",
                leave=False
            )

            for _ in skip_progress:
                next(data_iter)

        # ------------------------
        # 正式训练
        # ------------------------

        progress = tqdm(
            data_iter,
            desc="Training",
            leave=False,
            initial=skip_batches,
            total=total_batches
        )

        for batch_idx, batch in enumerate(progress, start=skip_batches):

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