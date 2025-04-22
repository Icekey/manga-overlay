use log::{info, warn};
use ort::execution_providers::{CUDAExecutionProvider, ExecutionProvider};
use ort::session::builder::SessionBuilder;
use ort::session::Session;

pub fn create_session_builder() -> anyhow::Result<SessionBuilder> {
    let mut builder = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?;

    let cuda = CUDAExecutionProvider::default();
    if cuda.is_available()? {
        info!("CUDA is available");
    } else {
        warn!("CUDA is not available");
    }

    let result = cuda.register(&mut builder);
    if result.is_err() {
        warn!("Failed to register CUDA! {}", result.unwrap_err());
    } else {
        info!("Registered CUDA");
    }

    Ok(builder)
}

#[test]
fn is_cuda_working() -> anyhow::Result<()> {
    let mut builder = Session::builder()?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?;

    let cuda = CUDAExecutionProvider::default();
    assert!(cuda.is_available().is_ok());

    let result = cuda.register(&mut builder);
    dbg!(&result);
    assert!(result.is_ok());

    Ok(())
}