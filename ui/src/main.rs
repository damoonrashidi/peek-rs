mod tools;

use cliclack::{Input, select, spinner};
use colored::Colorize;
use comfy_table::Table;
use db::Database;
use std::io::{self, Write};

use crate::tools::query_tool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let loading = spinner();
    loading.start("Loading LLM...");
    let mut llm = ai::LLM::new().await;
    loading.stop("Done!");
    llm.add_tool(query_tool());

    let conf = config::PeekConfig::get_or_default();

    let connection_options = conf
        .workspaces
        .iter()
        .flat_map(|workspace| {
            workspace
                .connections
                .iter()
                .map(|connection| {
                    (
                        connection.url.clone(),
                        format!("[{}] {}", workspace.name.clone(), connection.name.clone()),
                        connection.url.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let db_url = select("Select a connection")
        .filter_mode()
        .items(&connection_options)
        .interact()?;

    let mut database = db::postgres::PostgresDatabase::new(db_url).await;
    let schema = database.get_schema().await.unwrap();

    llm.set_system_prompt(format!(
        r#"
You are a database expert and you have been tasked at helping with database queries as well
as analysing results. You are currently working with a postgres database that has the following
schema {schema:?}. The schema consists of table names and columns,
as well as references (from table.col => [table.col])"#
    ))
    .await;

    while let Ok(prompt) = Input::new("You: ")
        .validate(|value: &String| {
            if value.is_empty() {
                return Err("Prompt cannot be empty");
            }
            Ok(())
        })
        .interact::<String>()
    {
        print!("\n[{}]", "[Assistant]".blue());

        let result = llm
            .stream_completion(prompt, |chunk| async move {
                match chunk {
                    ai::StreamChunk::Text(text) if !text.starts_with("<tool_call>") => {
                        print!("{}", text.blue());
                        let _ = io::stdout().flush();
                    }
                    ai::StreamChunk::ToolCall(tool_call) => {
                        println!(
                            "\n{}",
                            format!("[Calling tool: {}]", tool_call.name).yellow()
                        );
                    }
                    _ => {}
                }
            })
            .await;

        println!("\n");

        match result {
            Ok(tool_calls) if !tool_calls.is_empty() => {
                for tool_call in tool_calls {
                    println!("{}", format!("[{}]", tool_call.name).yellow());

                    let tool_result = match tool_call.name.as_str() {
                        "execute_query" => {
                            match serde_json::from_str::<serde_json::Value>(&tool_call.arguments) {
                                Ok(args) => {
                                    if let Some(query) = args
                                        .get("query")
                                        .and_then(|v: &serde_json::Value| v.as_str())
                                    {
                                        println!("{}", format!("Running query: {}", query).cyan());
                                        match database.get_results(query).await {
                                            Ok(results) => {
                                                let mut table = Table::new();
                                                table.set_header(
                                                    results
                                                        .headers
                                                        .iter()
                                                        .map(|header| header.0.clone()),
                                                );
                                                for row in results.rows.iter() {
                                                    table
                                                        .add_row(row.iter().map(|r| r.to_string()));
                                                }

                                                println!("{table}");
                                                format!("{results:?}")
                                            }
                                            Err(e) => format!("Error executing query: {e}"),
                                        }
                                    } else {
                                        "Error: No query parameter provided".to_string()
                                    }
                                }
                                Err(e) => format!("Error parsing arguments: {e}"),
                            }
                        }
                        _ => format!("Unknown tool: {}", tool_call.name),
                    };

                    print!("{}", "[Assistant]".blue());
                    if let Err(e) = llm
                        .add_tool_result(tool_call.id, tool_result, |chunk| async move {
                            if let ai::StreamChunk::Text(text) = chunk {
                                print!("{}", text.blue());
                                let _ = io::stdout().flush();
                            }
                        })
                        .await
                    {
                        eprintln!("Error adding tool result: {}", e);
                    }
                    println!();
                }
            }
            Err(err) => {
                eprintln!("{}", err);
            }
            _ => {}
        }
    }

    Ok(())
}
