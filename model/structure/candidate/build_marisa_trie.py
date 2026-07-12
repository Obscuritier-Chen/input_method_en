import sys#强行引入外层目录module
from pathlib import Path

current_file=Path(__file__).resolve()
outer_folder=current_file.parent.parent

if str(outer_folder) not in sys.path:
    sys.path.insert(0, str(outer_folder))

from word_vocabulary import WordVocabulary
from marisa_candidate_index import MarisaCandidateIndex
 

WORD2ID_PATH = r"model/datasets/vocabulary/word2id.json"

OUTPUT_PATH = r"model/structure/candidate/candidate.marisa"


def main():

    vocab = WordVocabulary(
        WORD2ID_PATH
    )

    candidate = MarisaCandidateIndex()

    candidate.build(vocab)

    candidate.save(
        OUTPUT_PATH
    )

    print("Finished.")


if __name__ == "__main__":

    main()