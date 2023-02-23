use apollo_parser::{ast::*, Parser};
use apollo_encoder::{
  Argument, Document, Field, OperationDefinition, OperationType,
  Selection, SelectionSet, VariableDefinition, Type_, Value
};
use std::collections::HashMap;
use convert_case::*;

pub fn generate_all(input: &str) -> anyhow::Result<String> {
  generate_all_to_document(input).map(|x| x.to_string())
}

pub fn generate_all_to_document(input: &str) -> anyhow::Result<Document> {
  let parser = Parser::new(input);
  let ast = parser.parse();
  if ast.errors().len() > 0 {
    anyhow::bail!("parse error {}", ast.errors().map(|x| x.to_string() ).collect::<Vec<String>>().join("\n") ) 
  }

  let schema = ast.document();

  let schema_object_type_definitions: Vec<ObjectTypeDefinition> = schema.definitions()
    .filter_map(|d| if let Definition::ObjectTypeDefinition(x) = d { Some(x) } else { None } )
    .collect();

  let object_type_fields_lookup = object_type_fields_lookup(&schema_object_type_definitions);

  let mut queries = Document::new();

  for object_type in schema_object_type_definitions {
    let query_operation_type = match name_to_string(object_type.name()).as_deref() {
      Some("Query") => || OperationType::Query,
      Some("Mutation") => || OperationType::Mutation,
      _ => continue,
    };

    let Some(schema_operations) = object_type.fields_definition().map(|x| x.field_definitions() ) else { continue };

    for schema_operation in schema_operations {
      let Some(query_operation_name) = name_to_string(schema_operation.name()) else { continue };
      let Some(query_return_type) = schema_operation.ty().and_then(extract_typename) else { continue };
      let Some(query_return_type_fields) = object_type_fields_lookup.get(&query_return_type) else { continue };

      let mut query_return = Field::new(query_operation_name.clone());

      let mut selections = vec![];
      for (top_level, maybe_nested) in query_return_type_fields {
        let mut field = Field::new(top_level.to_string());
        if let Some(nested) = maybe_nested {
          field.selection_set(Some(SelectionSet::with_selections(
            nested.iter().map(|f| Selection::Field(Field::new(f.to_string()))).collect()
          )));
        }
        selections.push(Selection::Field(field));
      }
      query_return.selection_set(Some(SelectionSet::with_selections(selections)));

      let mut query_variables = vec![];

      if let Some(schema_args) = schema_operation.arguments_definition() {
        for arg in schema_args.input_value_definitions() {
          let Some(arg_name) = name_to_string(arg.name()) else { continue };
          let Some(arg_type) = arg.ty().and_then(in_type_to_out_type) else { continue };

          query_variables.push( VariableDefinition::new(arg_name.clone(), arg_type) );
          query_return.argument( Argument::new(arg_name.clone(), Value::Variable(arg_name)) );
        }
      }

      let mut query_operation = OperationDefinition::new(
        query_operation_type(),
        SelectionSet::with_selections(vec![Selection::Field(query_return)]),
      );

      query_operation.name(Some(query_operation_name.to_case(Case::UpperCamel)));

      for var in query_variables {
        query_operation.variable_definition(var);
      }

      queries.operation(query_operation);
    }
  }

  Ok(queries)
}

fn in_type_to_out_type(input: Type) -> Option<Type_> {
  match input {
    Type::NamedType(x) => Some(Type_::NamedType{ name: name_to_string(x.name())? }),
    Type::ListType(x) => Some(Type_::List{ ty: Box::new(in_type_to_out_type(x.ty()?)?) }),
    Type::NonNullType(x) => Some(
      if let Some(named) = x.named_type() {
        Type_::NonNull{ ty: Box::new( in_type_to_out_type(Type::NamedType(named))? ) }
      } else {
        Type_::NonNull{ ty: Box::new( in_type_to_out_type(Type::ListType(x.list_type()?))? ) }
      }
    )
  }
}

fn extract_typename(input: Type) -> Option<String> {
  match input {
    Type::NamedType(x) => name_to_string(x.name()),
    Type::ListType(x) => extract_typename(x.ty()?), 
    Type::NonNullType(x) => {
      if let Some(named) = x.named_type() {
        extract_typename(Type::NamedType(named))
      } else {
        extract_typename(Type::ListType(x.list_type()?))
      }
    }
  }
}

/* This lookup matches an Object type to all its fields that we want to return.
 * We generate 1 level of nested attributes for fields which are themselves of a known custom object type. */
fn object_type_fields_lookup(definitions: &[ObjectTypeDefinition]) -> HashMap<String, Vec<(String, Option<Vec<String>>)>> {
  let mut fields_for_type = HashMap::new();
  for object_type in definitions {
    let Some(type_name) = name_to_string(object_type.name()) else { continue };
    let Some(field_defs) = object_type.fields_definition().map(|x| x.field_definitions() ) else { continue };
    let mut fieldnames: Vec<(String, Option<Type>)> = field_defs.filter_map(|d|{
      name_to_string(d.name()).map(|name| (name, d.ty()) )
    }).collect();
    fieldnames.push(("__typename".to_string(), None));
    fields_for_type.insert(type_name, fieldnames);
  }

  let mut lookup = HashMap::new();
  for (object_name, fieldnames) in &fields_for_type {
    let mut deep_fieldnames = vec![];
    for (field, ty) in fieldnames {
      let children = ty.clone().and_then(extract_typename)
        .and_then(|name| fields_for_type.get(&name) )
        .map(|fields|{
          fields.iter().filter_map(|field|{
            if field.1.clone().and_then(extract_typename).map(|x| !fields_for_type.contains_key(&x) ).unwrap_or(true) {
              Some(field.0.clone())
            } else {
              None
            }
          }).collect()
        });

      deep_fieldnames.push((field.clone(), children));
    }
    lookup.insert(object_name.clone(), deep_fieldnames);
  }
  
  lookup
}

fn name_to_string(n: Option<Name>) -> Option<String> {
  n.map(|x| x.text().to_string() )
}

mod test {
  #[test]
  fn sanity_check() {
    assert_eq!(
      super::generate_all(&std::fs::read_to_string("fixtures/schema.graphql").unwrap()).unwrap(), 
      std::fs::read_to_string("fixtures/queries.graphql").unwrap()
    )
  }
}
