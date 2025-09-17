use autonomi::client::key_derivation::{DerivationIndex, MainSecretKey};
use autonomi::client::payment::{PaymentOption, Receipt};
use autonomi::client::quote::{DataTypes, StoreQuote};
use autonomi::vault::user_data::USER_DATA_VAULT_CONTENT_IDENTIFIER;
use autonomi::vault::{
    vault_split_bytes, VaultError, VaultSecretKey, NUM_OF_SCRATCHPADS_PER_GRAPH_ENTRY,
    VAULT_HEAD_DERIVATION_INDEX,
};
use autonomi::{Bytes, Client, GraphEntry, PublicKey, Scratchpad, ScratchpadAddress};
use tracing::info;

#[derive(Debug, Clone)]
pub struct VaultQuoteResult {
    pub quote: StoreQuote,
    pub new_graph_entries: Vec<GraphEntry>,
    pub new_scratchpad_derivations: Vec<(PublicKey, [u8; 32])>,
}

#[derive(Debug, Clone, Default)]
pub struct VaultUpdate {
    pub new_graph_entries: Vec<GraphEntry>,
    pub new_scratchpad_derivations: Vec<(PublicKey, [u8; 32])>,
}

pub async fn vault_quote(
    client: &Client,
    data: Bytes,
    secret_key: &VaultSecretKey,
) -> Result<VaultQuoteResult, VaultError> {
    let main_secret_key = MainSecretKey::new(secret_key.clone());

    // scratchpad_derivations ordered by the collection order
    let (mut cur_free_graph_entry_derivation, mut scratchpad_derivations) = client
        .vault_claimed_capacity(
            &main_secret_key,
            DerivationIndex::from_bytes(VAULT_HEAD_DERIVATION_INDEX),
        )
        .await?;

    let contents = vault_split_bytes(data);

    info!(
        "Current capacity is {}, meanwhile requiring {}",
        scratchpad_derivations.len(),
        contents.len()
    );

    let mut quote: StoreQuote = StoreQuote(Default::default());
    let mut new_graph_entries: Vec<GraphEntry> = Vec::new();
    let mut new_scratchpad_derivations: Vec<(PublicKey, [u8; 32])> = Vec::new();

    while scratchpad_derivations.len() < contents.len() {
        let own_secret_key = main_secret_key.derive_key(&cur_free_graph_entry_derivation);

        let parents = vec![];
        let initial_value = [0u8; 32];

        // Generate random derivation for the next graph entry
        let new_graph_entry_derivation = DerivationIndex::random(&mut rand::thread_rng());

        let public_key: PublicKey = main_secret_key
            .derive_key(&new_graph_entry_derivation)
            .public_key()
            .into();

        let mut descendants = vec![(public_key, new_graph_entry_derivation.into_bytes())];

        descendants.extend((0..NUM_OF_SCRATCHPADS_PER_GRAPH_ENTRY).map(|_| {
            let derivation_index = DerivationIndex::random(&mut rand::thread_rng());

            let public_key: PublicKey = main_secret_key
                .derive_key(&derivation_index)
                .public_key()
                .into();

            (public_key, derivation_index.into_bytes())
        }));

        let graph_entry = GraphEntry::new(
            &own_secret_key.into(),
            parents,
            initial_value,
            descendants.clone(),
        );

        let graph_entry_address = graph_entry.address();
        let graph_entry_xor_name = graph_entry_address.xorname();

        let graph_entry_quote = client
            .get_store_quotes(
                DataTypes::GraphEntry,
                std::iter::once((graph_entry_xor_name, graph_entry.size())),
            )
            .await?;

        quote.0.extend(graph_entry_quote.0);

        let scratchpads = descendants.split_off(1);

        new_graph_entries.push(graph_entry);
        new_scratchpad_derivations.extend(&scratchpads);

        cur_free_graph_entry_derivation = new_graph_entry_derivation;
        scratchpad_derivations.extend(&scratchpads);
    }

    for (i, content) in contents.into_iter().enumerate() {
        let sp_secret_key =
            main_secret_key.derive_key(&DerivationIndex::from_bytes(scratchpad_derivations[i].1));

        let client = client.clone();

        let target_addr = ScratchpadAddress::new(sp_secret_key.public_key().into());

        info!("Checking existence of Scratchpad at {target_addr:?}...");

        if !client.scratchpad_check_existence(&target_addr).await? {
            let scratchpad_quote = client
                .get_store_quotes(
                    DataTypes::Scratchpad,
                    std::iter::once((target_addr.xorname(), content.len())),
                )
                .await?;

            info!("Scratchpad at {target_addr:?} does not exist. Xorname is: {:?}, quote is {scratchpad_quote:?}", target_addr.xorname());

            quote.0.extend(scratchpad_quote.0);
        }
    }

    Ok(VaultQuoteResult {
        quote,
        new_graph_entries,
        new_scratchpad_derivations,
    })
}

pub async fn vault_update(
    client: &Client,
    data: Bytes,
    secret_key: &VaultSecretKey,
    receipt: Receipt,
    new_graph_entries: Vec<GraphEntry>,
    new_scratchpad_derivations: Vec<(PublicKey, [u8; 32])>,
) -> Result<(), VaultError> {
    let main_secret_key = MainSecretKey::new(secret_key.clone());

    // Get initial vault capacity
    let (_cur_free_graph_entry_derivation, mut scratchpad_derivations) = client
        .vault_claimed_capacity(
            &main_secret_key,
            DerivationIndex::from_bytes(VAULT_HEAD_DERIVATION_INDEX),
        )
        .await?;

    let contents = vault_split_bytes(data);

    info!(
        "Current capacity is {}, meanwhile requiring {}",
        scratchpad_derivations.len(),
        contents.len()
    );

    for graph_entry in new_graph_entries {
        let (_graph_cost, _addr) = client
            .graph_entry_put(graph_entry, PaymentOption::Receipt(receipt.clone()))
            .await?;
    }

    scratchpad_derivations.extend(&new_scratchpad_derivations);

    for (i, content) in contents.into_iter().enumerate() {
        let sp_secret_key =
            main_secret_key.derive_key(&DerivationIndex::from_bytes(scratchpad_derivations[i].1));
        let client = client.clone();

        let target_addr = ScratchpadAddress::new(sp_secret_key.public_key().into());

        let already_exists = client.scratchpad_check_existence(&target_addr).await?;

        if already_exists {
            info!(
                "Updating Scratchpad at {target_addr:?} with content of {} bytes",
                content.len()
            );

            client
                .scratchpad_update(
                    &sp_secret_key.clone().into(),
                    *USER_DATA_VAULT_CONTENT_IDENTIFIER,
                    &content,
                )
                .await?;

            info!(
                "Updated Scratchpad at {target_addr:?} with content of {} bytes",
                content.len()
            );
        } else {
            info!("Creating Scratchpad at {target_addr:?}");

            let counter = 0;

            let scratchpad = Scratchpad::new(
                &sp_secret_key.into(),
                *USER_DATA_VAULT_CONTENT_IDENTIFIER,
                &content,
                counter,
            );

            info!(
                "Looking for scratchpad xor name in receipt: {:?}",
                target_addr.xorname()
            );

            if !receipt.contains_key(&target_addr.xorname()) {
                panic!("No scratchpad xorname in the receipt");
            }

            let (cost, addr) = client
                .scratchpad_put(scratchpad, PaymentOption::Receipt(receipt.clone()))
                .await?;

            info!("Created Scratchpad at {addr:?} for cost {cost:?}");
        }
    }

    Ok(())
}
