import random
import torch
import torch.nn.functional as F
from tqdm import tqdm


class PredictionAnalyzer:

    def __init__(
        self,
        model,
        dataloader,
        word_vocab,
        device
    ):
        self.model = model
        self.dataloader = dataloader
        self.word_vocab = word_vocab
        self.device = device

    @torch.no_grad()
    def analyze(
        self,
        num_samples=20,
        topk=5,
        only_wrong=False
    ):

        self.model.eval()

        printed = 0

        top1 = 0
        top3 = 0
        top5 = 0
        total = 0

        for batch in tqdm(self.dataloader):

            context = batch["context"].to(self.device)
            prefix = batch["prefix"].to(self.device)
            labels = batch["labels"].to(self.device)

            logits = self.model(
                context,
                prefix
            )

            prob = F.softmax(
                logits,
                dim=-1
            )

            values, indices = torch.topk(
                prob,
                k=topk,
                dim=-1
            )

            bs = labels.size(0)

            for i in range(bs):

                total += 1

                pred = indices[i]

                gt = labels[i].item()

                if gt == pred[0].item():
                    top1 += 1

                if gt in pred[:3]:
                    top3 += 1

                if gt in pred[:5]:
                    top5 += 1

                correct = gt == pred[0].item()

                if only_wrong and correct:
                    continue

                if printed >= num_samples:
                    continue

                printed += 1

                print("=" * 80)

                print("Ground Truth :",
                      self.word_vocab.id_to_word(gt))

                print()

                print("Top Prediction:")

                for rank in range(topk):

                    wid = pred[rank].item()

                    print(
                        f"{rank+1:2d}. "
                        f"{self.word_vocab.id_to_word(wid):20s}"
                        f"{values[i][rank].item():.4f}"
                    )

                print()

                print("Correct :", correct)

            if printed >= num_samples:
                break

        print("\n")
        print("=" * 80)

        print(f"Top1 Accuracy : {top1/total:.4f}")
        print(f"Top3 Accuracy : {top3/total:.4f}")
        print(f"Top5 Accuracy : {top5/total:.4f}")