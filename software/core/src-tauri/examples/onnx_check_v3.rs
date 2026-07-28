use std::fs;

use ort::session::Session;
use ort::value::Tensor;

use serde::Deserialize;

#[derive(Deserialize)]
struct TestCase {
    context_ids: Vec<Vec<i64>>,
    candidate_ids: Vec<Vec<i64>>,
    candidate_mask: Vec<Vec<bool>>,
    expected_logits: Vec<Vec<f32>>,
}

fn main() -> ort::Result<()> {

    // -------------------------
    // 1. 加载测试用例
    // -------------------------

    let json_text = fs::read_to_string("../../../model_export/output/test_case_v3.json")
        .expect("读取 test_case_v3.json 失败，检查相对路径是否正确");

    let test_case: TestCase = serde_json::from_str(&json_text)
        .expect("解析 test_case_v3.json 失败");

    let batch = test_case.context_ids.len();
    let context_len = test_case.context_ids[0].len();
    let candidate_count = test_case.candidate_ids[0].len();

    // -------------------------
    // 2. 展平成一维数组，构造 ort Tensor
    // -------------------------

    let context_ids_flat: Vec<i64> = test_case.context_ids.into_iter().flatten().collect();
    let candidate_ids_flat: Vec<i64> = test_case.candidate_ids.into_iter().flatten().collect();
    let candidate_mask_flat: Vec<bool> = test_case.candidate_mask.into_iter().flatten().collect();

    let context_ids_tensor = Tensor::from_array(([batch, context_len], context_ids_flat))?;
    let candidate_ids_tensor = Tensor::from_array(([batch, candidate_count], candidate_ids_flat))?;
    let candidate_mask_tensor = Tensor::from_array(([batch, candidate_count], candidate_mask_flat))?;

    // -------------------------
    // 3. 加载模型，跑推理
    // -------------------------

    let mut session = Session::builder()?
        .commit_from_file("../../../model_export/output/completion_model_v3.onnx")?;

    let outputs = session.run(ort::inputs![
        "context_ids" => context_ids_tensor,
        "candidate_ids" => candidate_ids_tensor,
        "candidate_mask" => candidate_mask_tensor,
    ])?;

    let logits = outputs["logits"].try_extract_tensor::<f32>()?;

    // -------------------------
    // 4. 与 Python 期望输出比对
    // -------------------------

    let expected_flat: Vec<f32> = test_case.expected_logits.into_iter().flatten().collect();

    let logits_flat: Vec<f32> = logits.1.to_vec();

    if logits_flat.len() != expected_flat.len() {
        println!(
            "形状不匹配！rust 输出长度 {}，python 期望长度 {}",
            logits_flat.len(),
            expected_flat.len()
        );
        return Ok(());
    }

    let max_diff = logits_flat
        .iter()
        .zip(expected_flat.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);

    println!("logits shape: {:?}", logits.0);
    println!("candidate_count: {candidate_count}");
    println!("Max abs diff (rust ort vs python torch): {max_diff:.8}");

    if max_diff < 1e-4 {
        println!("PASS: 数值一致性校验通过");
    } else {
        println!("WARNING: 差异较大，请检查输入构造或模型导出是否正确");
    }

    Ok(())
}