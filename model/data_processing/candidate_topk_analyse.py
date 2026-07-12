import json
import bisect
from collections import defaultdict

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

DATASET_PATH = "model/datasets/processed/valid.jsonl"

TOPK_LIST = [16, 32, 64, 128, 256]


def main():

    ############################################################
    # Load
    ############################################################

    print("Loading vocabulary...")

    vocab = WordVocabulary(VOCAB_PATH)

    print("Loading candidate trie...")

    candidate = MarisaCandidateIndex()

    candidate.load(
        TRIE_PATH,
        vocab
    )

    ############################################################
    # First Pass:
    # Collect unique prefixes
    ############################################################

    print()
    print("Collecting unique prefixes...")

    prefixes = set()

    with open(
        DATASET_PATH,
        "r",
        encoding="utf8"
    ) as f:

        for line in tqdm(f):

            sample = json.loads(line)

            prefix = sample["prefix"].lower()

            prefixes.add(prefix)

    print()

    print(
        f"Unique prefixes: {len(prefixes)}"
    )

    ############################################################
    # Build Cache
    ############################################################

    print()
    print("Building prefix cache...")

    """
    prefix_cache[prefix] = {

        "rank": {
            word_id : rank
        },

        "count": total_candidate_count
    }
    """

    prefix_cache = {}

    for prefix in tqdm(prefixes):

        ########################################################
        # Retrieve all candidates
        ########################################################

        words = candidate.lookup_words(prefix)

        ########################################################
        # Sort by frequency
        #
        # Smaller id => Higher frequency
        ########################################################

        words.sort(
            key=vocab.word_to_id
        )

        ########################################################
        # Encode
        ########################################################

        ids = vocab.encode(words)

        ########################################################
        # Build Rank Map
        ########################################################

        rank_map = {}

        for rank, word_id in enumerate(ids):

            rank_map[word_id] = rank

        ########################################################

        prefix_cache[prefix] = {

            "rank": rank_map,

            "count": len(ids)

        }

    ############################################################
    # Statistics
    ############################################################

    total = 0

    oov = 0

    ############################################################
    # Overall Recall@K
    ############################################################

    hit = {

        k: 0

        for k in TOPK_LIST

    }

    ############################################################
    # Prefix Length Recall
    ############################################################

    length_total = defaultdict(int)

    length_hit = {

        k: defaultdict(int)

        for k in TOPK_LIST

    }

    ############################################################
    # Candidate Count
    ############################################################

    candidate_count = defaultdict(list)

    ############################################################
    # Second Pass:
    # Actual Evaluation
    ############################################################

    print()
    print("Running evaluation...")

    with open(
        DATASET_PATH,
        "r",
        encoding="utf8"
    ) as f:

        for line in tqdm(f):

            sample = json.loads(line)

            prefix = sample["prefix"].lower()

            target = sample["target"].lower()

            ####################################################
            # OOV
            ####################################################

            if target not in vocab.word2id:

                oov += 1

                continue

            ####################################################

            total += 1

            ####################################################
            # Prefix Length
            ####################################################

            length = len(prefix)

            length_total[length] += 1

            ####################################################
            # Candidate Count
            ####################################################

            cache = prefix_cache[prefix]

            candidate_count[length].append(
                cache["count"]
            )

            ####################################################
            # Rank
            ####################################################

            target_id = vocab.word_to_id(target)

            rank = cache["rank"].get(
                target_id,
                None
            )

            ####################################################
            # Recall@K
            ####################################################

            if rank is not None:

                for k in TOPK_LIST:

                    if rank < k:

                        hit[k] += 1

                        length_hit[k][length] += 1

    ############################################################
    # Print Overall Recall
    ############################################################

    print()

    print("=" * 60)

    print("Overall Recall@K")

    print("=" * 60)

    print(f"Valid Samples : {total}")

    print(f"OOV Samples   : {oov}")

    print()

    for k in TOPK_LIST:

        recall = hit[k] / total * 100

        print(

            f"Top{k:<4}: "

            f"{recall:.2f}%"

        )

    ############################################################
    # Print Prefix Length Recall
    ############################################################

    print()

    print("=" * 100)

    print("Recall by Prefix Length")

    print("=" * 100)

    header = (

        "Len".ljust(8)

    )

    for k in TOPK_LIST:

        header += f"Top{k}".rjust(12)

    header += "AvgCand".rjust(14)

    print(header)

    print("-" * 100)

    ############################################################

    for length in sorted(length_total.keys()):

        line = f"{length:<8}"

        ########################################################
        # Recall@K
        ########################################################

        for k in TOPK_LIST:

            rate = (

                length_hit[k][length]

                / length_total[length]

                * 100

            )

            line += f"{rate:.2f}%".rjust(12)

        ########################################################
        # Avg Candidate Count
        ########################################################

        avg = (

            sum(candidate_count[length])

            / len(candidate_count[length])

        )

        line += f"{avg:.1f}".rjust(14)

        ########################################################

        print(line)

    ############################################################
    # Average Rank
    ############################################################

    print()

    print("=" * 60)

    print("Analysis Finished")

    print("=" * 60)


if __name__ == "__main__":

    main()