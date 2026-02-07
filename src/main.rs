use ai::{HashMap, LLM, Tool, Value, create_tool, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the LLM instance
    let mut llm = LLM::new().await;

    // Define tools specific to this application
    let query_database_tool = create_query_database_tool();
    let generate_query_tool = create_generate_query_tool();

    // Add tools to the LLM
    llm.add_tool(query_database_tool);
    llm.add_tool(generate_query_tool);

    // Set a system prompt
    llm.set_system_prompt("You are a helpful database assistant. Use the provided tools to help users with their database queries.").await;

    // Stream a completion with tool support
    println!("Asking the LLM to query the database...\n");
    llm.stream_completion("What tables are in the database?", |chunk| async move {
        if let ai::StreamChunk::Text(text) = chunk {
            print!("{}", text)
        }
    })
    .await?;

    Ok(())
}

// Example: Create a tool for querying the database
fn create_query_database_tool() -> Tool {
    let parameters: HashMap<String, Value> = serde_json::from_value(json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "The SQL query to execute against the database.",
            },
        },
        "required": ["query"],
    }))
    .expect("Invalid tool parameters");

    create_tool(
        "query_database",
        "Execute a SQL query against the database and return the results.",
        parameters,
    )
}

// Example: Create a tool for generating SQL queries
fn create_generate_query_tool() -> Tool {
    let parameters: HashMap<String, Value> = serde_json::from_value(json!({
        "type": "object",
        "properties": {
            "description": {
                "type": "string",
                "description": "A natural language description of what the SQL query should accomplish.",
            },
            "schema": {
                "type": "string",
                "description": "Optional database schema information to help generate the query.",
            },
        },
        "required": ["description"],
    }))
    .expect("Invalid tool parameters");

    create_tool(
        "generate_query",
        "Generate a SQL query based on a natural language description.",
        parameters,
    )
}
