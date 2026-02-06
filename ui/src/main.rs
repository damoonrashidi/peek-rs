use cliclack::{Input, select, spinner};
use colored::Colorize;
use db::Database;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let loading = spinner();
    loading.start("Loading LLM...");
    let mut llm = ai::LLM::new().await;
    loading.stop("Done!");

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
        print!("\n{}", "[Assistant]".blue());

        let result = llm
            .stream_completion(prompt, |chunk| async move {
                print!("{}", chunk.blue());
                let _ = io::stdout().flush();
            })
            .await;

        println!("\n");

        if let Err(err) = result {
            eprintln!("{err}");
        }
    }

    Ok(())
}
