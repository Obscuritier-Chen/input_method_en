import json

from tqdm import tqdm

import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

from structure.word_vocabulary import WordVocabulary
from structure.candidate.marisa_candidate_index import MarisaCandidateIndex


VOCAB_PATH = "model/datasets/vocabulary/word2id.json"

TRIE_PATH = "model/datasets/candidate/candidate.marisa"

OUTPUT_PATH = "model/datasets/candidate/prefix_candidates.json"

TOP_K = 64


def main():

    vocab = WordVocabulary(VOCAB_PATH)

    candidate = MarisaCandidateIndex()

    candidate.load(
        TRIE_PATH,
        vocab
    )

    table = {}

    print("Building Prefix Candidate Table...")

    for word in tqdm(vocab.word2id.keys()):

        if word.startswith("<"):

            continue

        for i in range(1, len(word)+1):

            prefix = word[:i]

            if prefix in table:

                continue

            candidate_words = candidate.lookup_words(prefix)

            candidate_words.sort(
                key=vocab.word_to_id
            )

            # ③ 保留 TopK
            candidate_words = candidate_words[:TOP_K]

            # ④ 转成 id
            table[prefix] = vocab.encode(candidate_words)

    print(

        f"Total Prefixes: {len(table)}"

    )

    with open(

        OUTPUT_PATH,

        "w",

        encoding="utf8"

    ) as f:

        json.dump(

            table,

            f

        )


if __name__ == "__main__":

    main()