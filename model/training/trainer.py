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
        device
    ):

        self.model = model.to(device)
        self.optimizer = optimizer
        self.criterion = criterion
        self.device = device

    def train_epoch(
        self,
        dataloader
    ):

        self.model.train()

        metrics = Metrics()

        progress = tqdm(
            dataloader,
            desc="Training",
            leave=False
        )

        for batch in progress:

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