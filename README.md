Generates queries for all object types defined in a GraphQL Schema.

This is useful as as starting point when building a catch-all compile time checked client, and you want to support every object with a query from the start, or for exhaustive testing purposes.

We used it at Constata to feed queries to our client built with https://github.com/graphql-rust/graphql-client

If you're looking for something similar to generate testing data in rust have a look at "apollo-smith" in https://github.com/apollographql/apollo-rs

A similar tool exists for javascript users in https://github.com/timqian/gql-generator

In true spirit of graphql, you shouldn't feed queries to your users. Always have a graphql explorer available for them to craft their own.

## Usage
Just check out the repo and `cargo run -- -i existing_schema.graphql -o your_queries.graphql`
