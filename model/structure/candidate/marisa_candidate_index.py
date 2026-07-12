import marisa_trie

from .candidate_index import CandidateIndex


class MarisaCandidateIndex(CandidateIndex):

    def __init__(self, word_vocab=None):

        self.trie = None
        self.word_vocab = word_vocab

    def build(self, word_vocab):

        self.word_vocab = word_vocab

        print("Building MARISA Trie...")

        words = [

            word.lower()

            for word in word_vocab.word2id.keys()

            if not word.startswith("<")

        ]

        self.trie = marisa_trie.Trie(words)

        print(f"Trie built. Total words: {len(words)}")

    def lookup(
        self,
        prefix,
        top_k=None
    ):

        if self.trie is None:

            raise RuntimeError(
                "Trie has not been built or loaded."
            )

        if self.word_vocab is None:

            raise RuntimeError(
                "word_vocab has not been assigned."
            )

        words = self.trie.keys(
            prefix.lower()
        )

        if top_k is not None:

            words = words[:top_k]

        return [

            self.word_vocab.word2id[word]

            for word in words

        ]

    def lookup_words(
        self,
        prefix,
        top_k=None
    ):

        if self.trie is None:

            raise RuntimeError(
                "Trie has not been built or loaded."
            )

        words = self.trie.keys(
            prefix.lower()
        )

        if top_k is not None:

            words = words[:top_k]

        return words

    def save(
        self,
        path
    ):

        self.trie.save(path)

    def load(
        self,
        path,
        word_vocab
    ):

        self.trie = marisa_trie.Trie()

        self.trie.load(path)

        self.word_vocab = word_vocab