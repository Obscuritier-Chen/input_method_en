fn main() -> anyhow::Result<()> {

    // 注意：把 core_service_lib 换成你实际的 lib crate 名称
    // （通常是 Cargo.toml 里 package name 把 "-" 换成 "_"，
    //  比如 package name 是 "core-service" 则这里是 core_service_lib，
    //  具体看 [lib] name 字段，如果没单独指定就是 package name 转换后的结果）

    use core_service_lib::{WordVocabulary, CandidateIndex, Predictor};

    let vocab = WordVocabulary::load("../../../model/datasets/vocabulary/word2id.json")?;

    let candidate_index = CandidateIndex::load(
        "../../../model/datasets/candidate/prefix_candidates.json"
    )?;

    let mut predictor = Predictor::load(
        "../../../model_export/output/completion_model_v3.onnx"
    )?;

    println!("vocab size: {}", vocab.len());
    println!("candidate table size: {}", candidate_index.len());

    // -------------------------
    // 模拟一次真实输入：左侧上下文 + 用户正在打的前缀
    // -------------------------

    let context_words = ["i", "am", "a", "fucking"];
    let prefix = "id";  // 假设正在打 "school"

    let context_ids = vocab.encode(&context_words);

    let candidate_ids = candidate_index
        .get_candidates(prefix)
        .cloned()
        .unwrap_or_default();

    if candidate_ids.is_empty() {
        println!("前缀 '{prefix}' 没有查到候选词，换一个前缀试试");
        return Ok(());
    }

    println!("前缀 '{prefix}' 查到 {} 个候选", candidate_ids.len());

    let results = predictor.predict(&context_ids, &candidate_ids, &vocab, 5)?;

    println!("Top-5 预测结果：");

    for (rank, candidate) in results.iter().enumerate() {
        println!("{}. {} (score: {:.4})", rank + 1, candidate.word, candidate.score);
    }

    Ok(())
}