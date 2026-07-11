import time
import torch


class Metrics:

    def __init__(self):

        self.reset()

    def reset(self):

        self.total_loss = 0.0

        self.total_top1 = 0.0
        self.total_top5 = 0.0

        self.total_mrr = 0.0

        self.total_latency = 0.0

        self.total_samples = 0

    @torch.no_grad()
    def update_prediction(
        self,
        logits,
        labels,
        loss
    ):

        batch_size = labels.size(0)

        self.total_loss += loss.item() * batch_size

        # ------------------------
        # Top1
        # ------------------------

        top1 = torch.topk(
            logits,
            k=1,
            dim=1
        ).indices

        top1 = (
            top1.squeeze(1)
            == labels
        ).float().mean().item()

        # ------------------------
        # Top5
        # ------------------------

        top5 = torch.topk(
            logits,
            k=5,
            dim=1
        ).indices

        top5 = top5.eq(
            labels.unsqueeze(1)
        ).any(dim=1).float().mean().item()

        # ------------------------
        # MRR
        # ------------------------

        ranking = torch.argsort(
            logits,
            dim=1,
            descending=True
        )

        rank = (
            ranking
            ==
            labels.unsqueeze(1)
        ).nonzero(
            as_tuple=False
        )[:, 1]

        mrr = (
            1.0 /
            (rank.float() + 1.0)
        ).mean().item()

        # ------------------------

        self.total_top1 += top1 * batch_size

        self.total_top5 += top5 * batch_size

        self.total_mrr += mrr * batch_size

        self.total_samples += batch_size

    def update_latency(
        self,
        elapsed_time
    ):

        self.total_latency += elapsed_time

    def compute(self):

        return {

            "loss":

                self.total_loss
                /
                self.total_samples,

            "top1":

                self.total_top1
                /
                self.total_samples,

            "top5":

                self.total_top5
                /
                self.total_samples,

            "mrr":

                self.total_mrr
                /
                self.total_samples,

            "latency_ms":

                (
                    self.total_latency
                    /
                    self.total_samples
                )
                *1000,

            "throughput":

                (
                    self.total_samples
                    /
                    self.total_latency
                )

        }