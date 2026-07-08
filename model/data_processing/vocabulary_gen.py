import json
from pathlib import Path
from collections import Counter

# =====================================
# Configuration
# =====================================

TRAIN_FILE = r"./model/datasets/processed/train.jsonl"

OUTPUT_DIR = r"./model/datasets/vocabulary"

MIN_FREQUENCY = 1          # 建议Baseline直接保留全部词
LOWER_CASE = True

SPECIAL_TOKENS = [
    "<PAD>",
    "<UNK>"
]

# =====================================
# Count word frequency (With Progress)
# =====================================

counter = Counter()

print("=== 开始统计词频 ===")
processed_lines = 0

# 🎯 优化：在遍历时顺便统计 max 和 sum，避免最后全量遍历 values()
total_tokens_count = 0
max_word_frequency = 0

with open(TRAIN_FILE, "r", encoding="utf-8") as f:
    for line in f:
        if not line.strip():
            continue
            
        try:
            sample = json.loads(line)
            word = sample["target"]

            if LOWER_CASE:
                word = word.lower()

            counter[word] += 1
            
            # 实时进度追踪
            processed_lines += 1
            if processed_lines % 1000000 == 0:
                print(f"[进度提示] 已读取样本行数: {processed_lines:,} | 当前唯一词数: {len(counter):,}")
                
        except json.JSONDecodeError:
            continue
        except KeyError:
            # 预防某些损坏样本没有 "target" 键
            continue

print(f"-> 词频统计完成。总读取有效行数: {processed_lines:,}")
print(f"-> 唯一词总数 (Unique words): {len(counter):,}")

# =====================================
# Build vocabulary
# =====================================

print("\n=== 开始构建词表 ===")
vocabulary = []

# Special tokens
for token in SPECIAL_TOKENS:
    vocabulary.append({
        "word": token,
        "id": len(vocabulary),
        "frequency": -1,
        "length": 0
    })

# Sort by frequency
print("正在对词频进行降序排序...")
sorted_words = sorted(
    counter.items(),
    key=lambda x: (-x[1], x[0])
)

print("正在填充词表结构...")
for word, freq in sorted_words:
    if freq < MIN_FREQUENCY:
        continue

    # 🎯 优化：在排序结果中顺便捕获最高频和总频数，无任何额外开销
    if freq > max_word_frequency:
        max_word_frequency = freq
    total_tokens_count += freq

    vocabulary.append({
        "word": word,
        "id": len(vocabulary),
        "frequency": freq,
        "length": len(word)
    })

print(f"-> 词表构建完成。最终词表大小 (含Special Tokens): {len(vocabulary):,}")

# =====================================
# Build lookup tables
# =====================================

print("\n=== 开始构建高效正反向索引表 ===")
word2id = {}
id2word = {}

for item in vocabulary:
    word = item["word"]
    idx = item["id"]
    word2id[word] = idx
    id2word[idx] = word

# =====================================
# Save (With explicit UTF-8)
# =====================================

print("\n=== 开始序列化落盘 ===")
output = Path(OUTPUT_DIR)
output.mkdir(parents=True, exist_ok=True)

# vocabulary.json
print("正在保存: vocabulary.json ...")
with open(output / "vocabulary.json", "w", encoding="utf-8") as f:
    json.dump(vocabulary, f, ensure_ascii=False, indent=4)

# word2id
print("正在保存: word2id.json ...")
with open(output / "word2id.json", "w", encoding="utf-8") as f:
    json.dump(word2id, f, ensure_ascii=False, indent=4)

# id2word
print("正在保存: id2word.json ...")
with open(output / "id2word.json", "w", encoding="utf-8") as f:
    json.dump(id2word, f, ensure_ascii=False, indent=4)

# vocabulary.txt
print("正在保存: vocabulary.txt ...")
with open(output / "vocabulary.txt", "w", encoding="utf-8") as f:
    for item in vocabulary:
        f.write(item["word"] + "\n")

# statistics
print("正在计算统计指标并保存: statistics.json ...")
# 🎯 修复：避免使用容易引发内存和CPU风暴的 max() 和 sum()，直接使用前面累加的值
statistics = {
    "vocabulary_size": len(vocabulary),
    "min_frequency": MIN_FREQUENCY,
    "unique_words": len(counter),
    "max_frequency": max_word_frequency,
    "average_frequency": total_tokens_count / len(counter) if len(counter) > 0 else 0
}

with open(output / "statistics.json", "w", encoding="utf-8") as f:
    json.dump(statistics, f, indent=4)

print("\n🎉 所有的词表文件已成功安全输出！Done.")