use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ae_ai_tools::inference::{InferenceConfig, InferenceSession, ModelType};
use ae_ai_tools::quantization::{QuantizationConfig, QuantizationMode, Quantizer};
use ae_ai_tools::rag::{KnowledgeSource, RagConfig, RagEngine};

fn bench_quantization(c: &mut Criterion) {
    let floats: Vec<f32> = (0..100000).map(|i| (i as f32) * 0.001).collect();

    let int8_config =
        QuantizationConfig { mode: QuantizationMode::Int8, group_size: 128, ..Default::default() };

    c.bench_function("ai_quantize_int8_100k", |bench| {
        bench.iter(|| {
            let _ = Quantizer::quantize_fp32_to_int8(black_box(&floats), &int8_config);
        });
    });

    let int4_config =
        QuantizationConfig { mode: QuantizationMode::Int4, group_size: 128, ..Default::default() };

    c.bench_function("ai_quantize_int4_100k", |bench| {
        bench.iter(|| {
            let _ = Quantizer::quantize_fp32_to_int4(black_box(&floats), &int4_config);
        });
    });
}

fn bench_rag_retrieval(c: &mut Criterion) {
    let config = RagConfig {
        embeddings_dim: 128,
        max_chunks: 2000,
        max_tokens: 1_000_000,
        top_k: 5,
        min_similarity: 0.0,
        context_max_tokens: 1024,
    };
    let mut engine = RagEngine::new(config);

    for i in 0..1000 {
        let embedding: Vec<f32> = (0..128).map(|j| ((i + j) as f32) * 0.001).collect();
        engine.ingest(
            format!("content_{}", i),
            KnowledgeSource::WorldData,
            0.5,
            vec!["test".to_string()],
        );
        engine.knowledge_base.chunks.last_mut().unwrap().embeddings = embedding;
    }

    c.bench_function("ai_rag_search_1000_docs", |bench| {
        let query: Vec<f32> = (0..128).map(|j| (j as f32) * 0.001).collect();
        bench.iter(|| {
            let results = engine.query(black_box(&query));
            black_box(results);
        });
    });
}

fn bench_inference_session(c: &mut Criterion) {
    let config = InferenceConfig {
        model_type: ModelType::Qwen,
        batch_size: 8,
        max_sequence_length: 2048,
        use_gpu: false,
        gpu_device_id: 0,
        num_threads: 4,
        warmup_iterations: 0,
        cache_activations: false,
    };

    c.bench_function("ai_inference_session_load", |bench| {
        bench.iter(|| {
            let mut session = InferenceSession::new(config.clone());
            let _ = session.load();
            black_box(session);
        });
    });
}

fn bench_inference_session_metrics(c: &mut Criterion) {
    let config = InferenceConfig {
        model_type: ModelType::Phi,
        batch_size: 1,
        max_sequence_length: 512,
        use_gpu: false,
        gpu_device_id: 0,
        num_threads: 1,
        warmup_iterations: 0,
        cache_activations: false,
    };
    let mut session = InferenceSession::new(config);
    session.load().unwrap();

    c.bench_function("ai_inference_record_metrics", |bench| {
        bench.iter(|| {
            session.record_inference(black_box(100), black_box(50));
        });
    });
}

criterion_group!(
    benches,
    bench_quantization,
    bench_rag_retrieval,
    bench_inference_session,
    bench_inference_session_metrics
);
criterion_main!(benches);
