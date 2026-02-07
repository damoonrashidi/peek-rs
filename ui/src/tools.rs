use ai::{create_tool, json, HashMap, Value};

pub fn query_tool() -> ai::Tool {
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
        "execute_query",
        "Execute a SQL query against the current database connection. Only use this tool when the user explicitly asks to run a query or needs to retrieve data from the database.",
        parameters,
    )
}
