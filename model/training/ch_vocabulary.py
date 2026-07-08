import json


class CharacterVocabulary:

    def __init__(self):

        self.special_tokens = [
            "<PAD>",
            "<UNK>"
        ]

        chars = list("abcdefghijklmnopqrstuvwxyz'-")

        self.id2char = self.special_tokens + chars

        self.char2id = {
            c: i
            for i, c in enumerate(self.id2char)
        }

    @property
    def pad_id(self):
        return self.char2id["<PAD>"]

    @property
    def unk_id(self):
        return self.char2id["<UNK>"]

    def encode(self, text):

        ids = []

        for ch in text.lower():

            ids.append(
                self.char2id.get(
                    ch,
                    self.unk_id
                )
            )

        return ids

    def decode(self, ids):

        return "".join(
            self.id2char[i]
            for i in ids
        )

    def __len__(self):

        return len(self.id2char)
    
if __name__=='__main__':
    voc=CharacterVocabulary()

    print(len(voc))

    print(voc.encode('testing~'))

    print(voc.decode([3,3,4,4]))