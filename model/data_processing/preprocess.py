import json
import random
import re
from pathlib import Path
import pandas as pd

# ===========================
# Configuration
# ===========================
INPUT_FILE = r"./model/datasets/raw_data/BookCorpus3.csv"
OUTPUT_DIR = r"./model/datasets/processed"
MAX_LEFT_WORDS = 64
VALID_RATIO = 0.01
TEST_RATIO = 0.01
RANDOM_SEED = 42

random.seed(RANDOM_SEED)

# 英文单词匹配
WORD_PATTERN = re.compile(r"[A-Za-z]+(?:['-][A-Za-z]+)*")
# 简单句子切分
SENTENCE_SPLIT = re.compile(r"[.!?]+")

# ===========================
# Functions
# ===========================
def tokenize(sentence: str):
    return WORD_PATTERN.findall(sentence)

def split_sentences(text: str):
    return [s.strip() for s in SENTENCE_SPLIT.split(text) if s.strip()]

def generate_sample(words, idx):
    target = words[idx]
    if len(target) < 2:
        return None

    prefix_len = random.randint(1, len(target) - 1)
    left = words[max(0, idx - MAX_LEFT_WORDS):idx]
    right = words[idx + 1:]

    return {
        "left": " ".join(left),
        "right": " ".join(right),
        "prefix": target[:prefix_len],
        "target": target
    }

# ===========================
# Main
# ===========================
def main():
    Path(OUTPUT_DIR).mkdir(parents=True, exist_ok=True)

    # 初始化各数据集的计数器
    train_count = 0
    valid_count = 0
    test_count = 0
    processed_paragraphs = 0

    # 🎯 解决方案：同时打开三个输出文件，采用写入模式
    with open(Path(OUTPUT_DIR) / "train.jsonl", "w", encoding="utf-8") as train_f, \
         open(Path(OUTPUT_DIR) / "valid.jsonl", "w", encoding="utf-8") as valid_f, \
         open(Path(OUTPUT_DIR) / "test.jsonl", "w", encoding="utf-8") as test_f:

        print("开始流式处理大规模数据集...")

        # 🎯 优化点 1：使用 chunksize 分块读取 CSV，每次只载入 50,000 行，内存占用极低
        # 加上 usecols=[0] 确保只读取第一列，进一步节省内存
        chunk_iter = pd.read_csv(INPUT_FILE, usecols=[0], chunksize=50000)

        for chunk in chunk_iter:
            for paragraph in chunk.iloc[:, 0]:
                processed_paragraphs += 1
                
                if not isinstance(paragraph, str):
                    continue

                sentences = split_sentences(paragraph)
                for sentence in sentences:
                    words = tokenize(sentence)
                    if len(words) < 2:
                        continue

                    for idx in range(len(words)):
                        if random.random()>0.10:
                            continue
                        sample = generate_sample(words, idx)
                        if sample is None:
                            continue

                        # 随机分流
                        r = random.random()
                        # 🎯 优化点 2：不再 append 到列表，而是直接序列化成 JSON 字符串写入磁盘
                        if r < TEST_RATIO:
                            test_f.write(json.dumps(sample, ensure_ascii=False) + "\n")
                            test_count += 1
                        elif r < TEST_RATIO + VALID_RATIO:
                            valid_f.write(json.dumps(sample, ensure_ascii=False) + "\n")
                            valid_count += 1
                        else:
                            train_f.write(json.dumps(sample, ensure_ascii=False) + "\n")
                            train_count += 1

            # 每处理完一个 chunk 打印一次进度日志
            print(f"已处理段落（行数）: {processed_paragraphs:,} | 当前训练集样本数: {train_count:,}")

    print("\n=== 数据集切分与处理完成 ===")
    print(f"Train 最终样本数 : {train_count:,}")
    print(f"Valid 最终样本数 : {valid_count:,}")
    print(f"Test  最终样本数 : {test_count:,}")

if __name__ == "__main__":
    main()