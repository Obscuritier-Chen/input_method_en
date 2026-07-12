from abc import ABC, abstractmethod


class CandidateIndex(ABC):

    @abstractmethod
    def build(self, word_vocab):
        """
        根据 Vocabulary 建立索引
        """
        pass

    @abstractmethod
    def lookup(
        self,
        prefix: str,
        top_k=None
    ):
        """
        Parameters
        ----------
        prefix

        top_k
            None 表示全部返回

        Returns
        -------
        List[int]
            word ids
        """
        pass

    @abstractmethod
    def save(self, path):
        pass

    def load(self, path):
        pass