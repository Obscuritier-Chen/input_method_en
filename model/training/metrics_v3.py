import torch


class MetricsV3:


    def __init__(
        self,
        top_k=(1,5,10)
    ):

        self.top_k = top_k



    @torch.no_grad()
    def __call__(
        self,
        logits,
        labels
    ):
        """
        Parameters
        ----------
        logits:
            [B,K]

        labels:
            [B]

        Returns
        -------
        dict
        """


        result = {}

        batch_size = labels.size(0)


        ##################################################
        # Ranking
        ##################################################

        sorted_indices = torch.argsort(

            logits,

            dim=-1,

            descending=True

        )


        ##################################################
        # Top-K Accuracy
        ##################################################

        for k in self.top_k:


            topk = sorted_indices[:, :k]


            correct = (

                topk == labels.unsqueeze(1)

            ).any(
                dim=1
            )


            result[
                f"top{k}_acc"
            ] = (

                correct.float().sum()

                /

                batch_size

            ).item()



        ##################################################
        # MRR
        ##################################################

        target_position = (

            sorted_indices

            ==

            labels.unsqueeze(1)

        )


        ranks = target_position.nonzero()[:,1] + 1


        result["mrr"] = (

            1.0 / ranks.float()

        ).mean().item()


        result["mean_rank"] = (

            ranks.float()

        ).mean().item()
        
        ##################################################
        # Mean Rank
        ##################################################

        result["mean_rank"] = (

            ranks.mean()

            .item()

        )


        return result