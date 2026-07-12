import json

from tqdm import tqdm

import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

from structure.word_vocabulary import WordVocabulary


VOCAB_PATH = "model/datasets/vocabulary/word2id.json"

PREFIX_TABLE = "model/datasets/candidate/prefix_candidates.json"

INPUT = "model/datasets/processed/test.jsonl"

OUTPUT = "model/datasets/processed/test_v3.jsonl"


def main():

    vocab = WordVocabulary(

        VOCAB_PATH

    )

    with open(

        PREFIX_TABLE,

        encoding="utf8"

    ) as f:

        candidate_table = json.load(f)

    keep = 0

    skip_candidate = 0
    skip_oov = 0

    with open(

        INPUT,

        encoding="utf8"

    ) as fin, open(

        OUTPUT,

        "w",

        encoding="utf8"

    ) as fout:

        for line in tqdm(fin):

            sample = json.loads(line)

            prefix = sample["prefix"].lower()

            target_word = sample["target"].lower()

            if target_word not in vocab.word2id:

                skip_oov += 1
                continue

            target_id = vocab.word2id[target_word]

            candidate_ids = candidate_table.get(prefix, [])

            if target_id not in candidate_ids:

                skip_candidate += 1
                continue

            label = candidate_ids.index(

                target_id

            )

            output = {

                "left": sample["left"],

                "prefix": sample["prefix"],

                "label": label

            }

            fout.write(

                json.dumps(output)

                + "\n"

            )

            keep += 1

    print()

    print("keep:", keep)

    print("skip_oov:", skip_oov)
    print("skip_candidate:", skip_candidate)


if __name__ == "__main__":

    main()