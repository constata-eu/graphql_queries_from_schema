use graphql_queries_from_schema::generate_all;
use std::io::{Read, stdin};
use std::path::PathBuf;
use clap::{Parser, ArgGroup};

#[derive(Parser, Debug)]
#[command(author, version, about,
    long_about = "Generates simple queries for all object types defined on a graphql schema. \
      The generated queries can be used as a starting point for compile time checked graphql clients."
)]
#[clap(group(ArgGroup::new("input").required(true).args(&["stdin", "input_file"])))]
struct Args {
   /// Read GraphQL schema introspection in query language (not json) from stdin
   #[arg(short, long)]
   stdin: bool,

   /// GraphQL schema introspection in query language (not json) to read.
   #[arg(short, long)]
   input_file: Option<PathBuf>,

   /// Filename to generate queries to. Defaults to stdout.
   #[arg(short, long)]
   output_file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
  let args = Args::parse();

  let input = if let Some(path) = args.input_file {
    std::fs::read_to_string(path)?
  } else {
    let mut i = String::new();
    stdin().read_to_string(&mut i).unwrap();
    i
  };

  let output = generate_all(&input)?;

  if let Some(path) = args.output_file {
    std::fs::write(path, output)?;
  } else {
    println!("{}", output);
  }

  Ok(())
}
