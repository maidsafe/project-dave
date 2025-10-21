use autonomi::client::quote::StoreQuote;
use std::collections::HashMap;

pub fn combine_quotes(quotes: Vec<StoreQuote>) -> StoreQuote {
    let combined_map = quotes
        .into_iter()
        .flat_map(|quote| quote.0)
        .collect::<HashMap<_, _>>();

    StoreQuote(combined_map)
}
