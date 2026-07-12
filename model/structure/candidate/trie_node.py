class TrieNode:

    def __init__(self):

        # children: dict[str, TrieNode]
        self.children = {}

        # 是否是一个完整单词
        self.is_word = False

        # 当前单词对应的 vocabulary id
        self.word_id = None