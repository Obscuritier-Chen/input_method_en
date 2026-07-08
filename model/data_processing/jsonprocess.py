import json
import re
from pathlib import Path

# ===========================
# 配置路径（请根据实际情况修改）
# ===========================
INPUT_JSONL = r"./model/datasets/processed/train.jsonl"
OUTPUT_JSONL = r"./model/datasets/processed/train_split.jsonl"

# 预编译正则：匹配一个或多个空格，用来精准切分单词
# 使用正则可以自动过滤掉由于连击产生的连续空格
SPACES_PATTERN = re.compile(r"\s+")

def process_jsonl():
    input_path = Path(INPUT_JSONL)
    output_path = Path(OUTPUT_JSONL)
    
    if not input_path.exists():
        print(f"[错误] 找不到输入文件: {input_path}")
        return

    print("开始高效转换 JSONL 数据...")
    processed_count = 0

    # 🎯 核心优化：同时打开输入和输出文件流，逐行处理
    with open(input_path, "r", encoding="utf-8") as infile, \
         open(output_path, "w", encoding="utf-8") as outfile:
        
        for line in infile:
            if not line.strip():
                continue
                
            try:
                # 1. 解析单行 JSON
                sample = json.loads(line)
                
                # 2. 核心转换逻辑：将字符串 split 成为 list
                # 使用 strip() 预防首尾空格导致的空字符串 '', 使用 regexp 容错连续空格
                left_str = sample.get("left", "").strip()
                sample["left"] = SPACES_PATTERN.split(left_str) if left_str else []
                
                right_str = sample.get("right", "").strip()
                sample["right"] = SPACES_PATTERN.split(right_str) if right_str else []
                
                # 3. 立刻序列化并写入新文件，不留在内存中
                outfile.write(json.dumps(sample, ensure_ascii=False) + "\n")
                
                processed_count += 1
                if processed_count % 1000000 == 0:
                    print(f"已成功处理 {processed_count // 1000000} 百万条数据...")
                    
            except json.JSONDecodeError:
                print(f"[警告] 发现损坏的 JSON 行，已跳过。")
                continue
            except Exception as e:
                print(f"[错误] 处理时发生异常: {e}")
                break

    print(f"\n=== 转换完成！ ===")
    print(f"总计处理样本数: {processed_count:,}")
    print(f"新文件已保存至: {output_path}")

if __name__ == "__main__":
    process_jsonl()