use autonomi::{Amount, QuoteHash, RewardsAddress};
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub type OrderID = u16;
pub type Payment = (QuoteHash, RewardsAddress, Amount);

pub const IDLE_PAYMENT_TIMEOUT_SECS: u64 = 30;
const CHANEL_SIZE: usize = 128;

pub enum OrderMessage {
    Cancelled,
    Completed,
    KeepAlive,
}

#[derive(Serialize, Clone)]
pub struct PaymentOrder {
    pub id: OrderID,
    payments: Vec<Payment>,
    #[serde(skip)]
    confirmation_sender: Sender<OrderMessage>,
}

impl PaymentOrder {
    pub fn new(payments: Vec<Payment>, confirmation_sender: Sender<OrderMessage>) -> Self {
        Self {
            id: Self::generate_id(),
            payments,
            confirmation_sender,
        }
    }

    pub fn generate_id() -> OrderID {
        rand::thread_rng().gen::<u16>()
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
    pub fn create_order(&self, payments: Vec<Payment>) -> (PaymentOrder, Receiver<OrderMessage>) {
        let (sender, receiver) = channel(CHANEL_SIZE);

        let order = PaymentOrder::new(payments, sender);

        // optimization: compare with tokio mutex
        let mut orders = self.orders.lock().expect("Could not get lock on orders");

        orders.insert(order.id, order.clone());

        drop(orders);

        (order, receiver)
    }
}
