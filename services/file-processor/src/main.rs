use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use qdrant_client::{
    qdrant::{CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, VectorParamsBuilder},
    Qdrant,
};

use std::{
    env,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::broadcast;
use tracing::{info, warn, error};
use uuid::Uuid;
use walkdir::WalkDir;
use xlib::{
    app::tracing::init_tracing,
    client::{
        OpenAIClient, OpenAIClientConfig, PostgresClient, PostgresClientConfig,
    },
};

#[derive(Clone)]
struct ProcessorState {
    pub pg_client: Arc<PostgresClient>,
    pub openai_client: Arc<OpenAIClient>,
    pub qdrant_client: Arc<Qdrant>,
    pub processed_files: Arc<Mutex<Vec<String>>>,
}

async fn init_clients() -> Result<ProcessorState> {
    // Initialize database client
    let db_config = PostgresClientConfig {
        hostname: env::var("DATABASE_HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
        port: Some(5432),
        user: Some(env::var("DATABASE_USER").unwrap_or_else(|_| "postgres".to_string())),
        password: Some(env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "password".to_string())),
        db_name: "rag".to_string(),
    };
    let pg_client = PostgresClient::build(&db_config).await?;

    // Initialize OpenAI client
    let openai_config = OpenAIClientConfig {
        api_key: env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable is required"),
        base_url: None,
    };
    let openai_client = OpenAIClient::new(openai_config)?;

    // Initialize Qdrant client
    let qdrant_url = env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    let qdrant_client = Qdrant::from_url(&qdrant_url).build()?;

    Ok(ProcessorState {
        pg_client: Arc::new(pg_client),
        openai_client: Arc::new(openai_client),
        qdrant_client: Arc::new(qdrant_client),
        processed_files: Arc::new(Mutex::new(Vec::new())),
    })
}

async fn setup_qdrant_collection(qdrant_client: &Qdrant) -> Result<()> {
    let collection_name = "rag_documents";
    let vector_size = 1536; // text-embedding-3-small dimension

    // Check if collection exists
    if let Ok(collections) = qdrant_client.list_collections().await {
        if !collections
            .collections
            .iter()
            .any(|c| c.name == collection_name)
        {
            // Create collection
            if let Err(e) = qdrant_client
                .create_collection(
                    CreateCollectionBuilder::new(collection_name)
                        .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
                )
                .await
            {
                warn!("Failed to create collection: {}", e);
            } else {
                info!("Created new collection: {}", collection_name);
            }
        } else {
            info!("Collection {} already exists", collection_name);
        }
    }

    Ok(())
}

async fn process_file(state: &ProcessorState, file_path: &Path) -> Result<()> {
    let file_path_str = file_path.to_string_lossy().to_string();
    
    // Check if file is already processed
    {
        let processed = state.processed_files.lock().unwrap();
        if processed.contains(&file_path_str) {
            return Ok(());
        }
    }

    info!("Processing file: {}", file_path_str);

    // Read file content
    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read file {}: {}", file_path_str, e);
            return Err(e.into());
        }
    };

    // Skip empty files
    if content.trim().is_empty() {
        info!("Skipping empty file: {}", file_path_str);
        return Ok(());
    }

    // Chunk the content (simple chunking by lines for now)
    let chunks = chunk_text(&content, 1000);
    
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        if let Err(e) = process_chunk(state, &file_path_str, chunk, chunk_idx).await {
            error!("Failed to process chunk {} of file {}: {}", chunk_idx, file_path_str, e);
        }
    }

    // Mark file as processed
    {
        let mut processed = state.processed_files.lock().unwrap();
        processed.push(file_path_str.clone());
    }

    info!("Successfully processed file: {}", file_path_str);
    Ok(())
}

async fn process_chunk(
    state: &ProcessorState,
    file_path: &str,
    chunk: &str,
    chunk_idx: usize,
) -> Result<()> {
    // Create embedding
    let embedding = state.openai_client.create_embedding(chunk).await?;

    // Generate unique ID for the chunk
    let chunk_id = Uuid::new_v5(
        &Uuid::NAMESPACE_DNS,
        format!("{}_{}", file_path, chunk_idx).as_bytes(),
    );

    // Create point for Qdrant
    let point = PointStruct::new(
        chunk_id.to_string(),
        embedding,
        [
            ("document_id", chunk_id.to_string().into()),
            ("file_path", file_path.into()),
            ("chunk_index", (chunk_idx as i64).into()),
            ("content", chunk.into()),
        ],
    );

    // Insert into Qdrant
    state
        .qdrant_client
        .upsert_points(UpsertPointsBuilder::new("rag_documents", vec![point]).wait(true))
        .await?;

    info!("Processed chunk {} of file: {}", chunk_idx, file_path);
    Ok(())
}

fn chunk_text(text: &str, max_chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    
    for line in text.lines() {
        if current_chunk.len() + line.len() + 1 > max_chunk_size && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }
        
        if !current_chunk.is_empty() {
            current_chunk.push('\n');
        }
        current_chunk.push_str(line);
    }
    
    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    
    chunks
}

async fn process_documents_directory(state: ProcessorState, documents_path: &str) -> Result<()> {
    info!("Processing documents directory: {}", documents_path);

    // Process existing files
    for entry in WalkDir::new(documents_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "txt" || ext == "md")
        })
    {
        if let Err(e) = process_file(&state, entry.path()).await {
            error!("Failed to process file {:?}: {}", entry.path(), e);
        }
    }

    // Set up file watcher for new files
    let (tx, mut rx) = broadcast::channel(100);
    
    let watcher_tx = tx.clone();
    let watcher_path = documents_path.to_string();
    
    tokio::task::spawn_blocking(move || {
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = watcher_tx.send(event);
                }
            },
            Config::default(),
        ).unwrap();

        watcher.watch(Path::new(&watcher_path), RecursiveMode::Recursive).unwrap();
        
        // Keep watcher alive
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    // Process file system events
    while let Ok(event) = rx.recv().await {
        use notify::EventKind;
        
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    if path.is_file() 
                        && path.extension().map_or(false, |ext| ext == "txt" || ext == "md") 
                    {
                        if let Err(e) = process_file(&state, &path).await {
                            error!("Failed to process file {:?}: {}", path, e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_tracing();

    info!("Starting file processor service...");

    let state = init_clients().await?;

    // Setup Qdrant collection
    if let Err(e) = setup_qdrant_collection(&state.qdrant_client).await {
        error!("Failed to setup Qdrant collection: {}", e);
        return Err(e);
    }

    // Get documents directory
    let documents_path = env::var("DOCUMENTS_PATH").unwrap_or_else(|_| "/documents".to_string());

    // Process documents
    if let Err(e) = process_documents_directory(state, &documents_path).await {
        error!("Failed to process documents directory: {}", e);
        return Err(e);
    }

    Ok(())
} 