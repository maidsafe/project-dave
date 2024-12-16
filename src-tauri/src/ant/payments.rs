use std::collections::{HashMap};
use std::sync::{Mutex};
use autonomi::{Amount, QuoteHash, RewardsAddress};
use serde::Serialize;
use tauri::async_runtime::{channel, Receiver, Sender};

pub type OrderID = String;
pub type Payment = (QuoteHash, RewardsAddress, Amount);

#[derive(Serialize, Clone)]
pub struct PaymentOrder {
    pub id: OrderID,
    payments: Vec<Payment>,
    #[serde(skip)]
    confirmation_sender: Sender<bool>
}

impl PaymentOrder {
    pub fn new(payments: Vec<Payment>, confirmation_sender: Sender<bool>) -> Self {
        Self {
            id: Self::generate_id(),
            payments,
            confirmation_sender,
        }
    }

    // todo: implement this
    pub fn generate_id() -> OrderID {
        "0000".to_string()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Default)]
pub struct PaymentOrderManager {
    orders: Mutex<HashMap<OrderID, PaymentOrder>>,
}

impl PaymentOrderManager {
    pub fn create_order(&self, payments: Vec<Payment>) -> (PaymentOrder, Receiver<bool>) {
        let (sender, receiver) = channel(1);

        let order = PaymentOrder::new(payments, sender);
        let mut orders = self.orders.lock().unwrap();

        orders.insert(order.id.clone(), order.clone());

        (order, receiver)
    }
}