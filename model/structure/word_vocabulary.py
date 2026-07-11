import json


class WordVocabulary:

    def __init__(self, path):

        with open(path, encoding="utf8") as f:

            self.word2id = json.load(f)

        self.id2word = {
            int(v): k
            for k, v in self.word2id.items()
        }

    def word_to_id(self, word):
        return self.word2id.get(
            word.lower(),
            self.word2id['<UNK>']
        )
    
    def id_to_word(self, idx):
        return self.id2word[idx]

    def encode(self, words):

        unk = self.word2id["<UNK>"]

        return [
            self.word2id.get(
                w.lower(),
                unk
            )
            for w in words
        ]

    def decode(self, ids):

        return [
            self.id2word[i]
            for i in ids
        ]

    def __len__(self):

        return len(self.word2id)
    
if __name__=='__main__':
    voc=WordVocabulary(r'./model/datasets/vocabulary/word2id.json')

    print(len(voc))

    print(voc.encode(['test', 'awad?a']))

    print(voc.decode([1,2,3,4]))

    print(voc.id_to_word(3))

    print(voc.word_to_id('test'))