import time
import torch
import torch.nn as nn
from tqdm import tqdm


class TrainerV3:

    def __init__(
        self,
        model,
        optimizer,
        device,
        metrics=None,
        scheduler=None,
        grad_clip=1.0,
        checkpoint_callback=None,   # 新增：每 save_interval 个 step 调用一次
        save_interval=50_000        # 新增：iteration 级别保存间隔
    ):

        self.model = model.to(device)

        self.optimizer = optimizer

        self.scheduler = scheduler

        self.device = device

        self.metrics = metrics

        self.grad_clip = grad_clip

        self.criterion = nn.CrossEntropyLoss()

        self.checkpoint_callback = checkpoint_callback
        self.save_interval = save_interval

        self.global_step = 0  # 跨 epoch 累计，断点恢复时从外部赋值恢复

    def train_epoch(
        self,
        loader,
        epoch=0,
        skip_batches=0
    ):

        self.model.train()

        total_loss = 0.0

        total_batches = len(loader)

        data_iter = iter(loader)

        # ------------------------
        # 手动逐个跳过，每步都推进 tqdm 显示
        # ------------------------

        if skip_batches > 0:

            skip_progress = tqdm(
                range(skip_batches),
                desc="Skipping (resume)",
                leave=False
            )

            for _ in skip_progress:
                next(data_iter)

        progress = tqdm(
            data_iter,
            desc="Training",
            leave=False,
            initial=skip_batches,
            total=total_batches
        )

        for batch_idx, batch in enumerate(progress, start=skip_batches):

            context_ids = batch["context_ids"].to(
                self.device
            )

            candidate_ids = batch["candidate_ids"].to(
                self.device
            )

            candidate_mask = batch["candidate_mask"].to(
                self.device
            )

            labels = batch["label"].to(
                self.device
            )

            ##################################################
            # Forward
            ##################################################

            logits = self.model(

                context_ids,

                candidate_ids,

                candidate_mask,

            )

            ##################################################
            # Loss
            ##################################################

            loss = self.criterion(

                logits,

                labels

            )

            ##################################################
            # Backward
            ##################################################

            self.optimizer.zero_grad()

            loss.backward()

            ##################################################
            # Gradient clipping
            ##################################################

            if self.grad_clip is not None:

                torch.nn.utils.clip_grad_norm_(

                    self.model.parameters(),

                    self.grad_clip

                )

            self.optimizer.step()

            if self.scheduler is not None:

                self.scheduler.step()

            total_loss += loss.item()

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
                    batch_in_epoch=batch_idx + 1,
                    loss=loss.item()
                )

            progress.set_postfix(

                loss=loss.item()

            )

        return total_loss / total_batches

    @torch.no_grad()
    def evaluate(
        self,
        loader
    ):

        self.model.eval()

        total_loss = 0.0

        metric_result = {}

        latency = 0.0

        sample_count = 0

        for batch in tqdm(
            loader,
            desc="Evaluation"
        ):

            context_ids = batch["context_ids"].to(
                self.device
            )

            candidate_ids = batch["candidate_ids"].to(
                self.device
            )

            candidate_mask = batch["candidate_mask"].to(
                self.device
            )

            labels = batch["label"].to(
                self.device
            )

            ##################################################
            # latency start
            ##################################################

            start = time.perf_counter()

            logits = self.model(

                context_ids,

                candidate_ids,

                candidate_mask,

            )

            ##################################################
            # latency end
            ##################################################

            latency += (
                time.perf_counter()
                -
                start
            )

            sample_count += context_ids.size(0)

            loss = self.criterion(

                logits,

                labels

            )

            total_loss += loss.item()

            ##################################################
            # Metrics
            ##################################################

            if self.metrics is not None:

                result = self.metrics(

                    logits,

                    labels

                )

                for k, v in result.items():

                    if k not in metric_result:

                        metric_result[k] = 0

                    metric_result[k] += v

        ##################################################
        # Average
        ##################################################

        avg_loss = total_loss / len(loader)

        if self.metrics is not None:

            for k in metric_result:

                metric_result[k] /= len(loader)

        metric_result["loss"] = avg_loss

        metric_result["latency_ms"] = (

            latency

            /

            sample_count

            *

            1000

        )

        return metric_result