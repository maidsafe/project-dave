use autonomi::{Amount, QuoteHash, RewardsAddress};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

pub type OrderID = u16;
pub type Payment = (QuoteHash, RewardsAddress, Amount);

pub const IDLE_PAYMENT_TIMEOUT_SECS: u64 = 600;
const CHANEL_SIZE: usize = 128;

#[derive(Serialize, Deserialize, Clone)]
pub enum OrderMessage {
    Cancelled,
    Completed,
    KeepAlive,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOrder {
    pub id: OrderID,
    pub payments: Vec<Payment>,
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
        let json = serde_json::to_string(self).unwrap();
        tracing::debug!("PaymentOrder JSON: {}", json);
        json
    }
}

#[derive(Default)]
pub struct PaymentOrderManager {
    orders: Mutex<HashMap<OrderID, PaymentOrder>>,
}

impl PaymentOrderManager {
    pub async fn create_order(
        &self,
        payments: Vec<Payment>,
    ) -> (PaymentOrder, Receiver<OrderMessage>) {
        let (sender, receiver) = channel(CHANEL_SIZE);

        let order = PaymentOrder::new(payments, sender);

        let mut orders = self.orders.lock().await;

        orders.insert(order.id, order.clone());

        (order, receiver)
    }

    pub async fn send_order_message(&self, id: OrderID, message: OrderMessage) {
        let mut orders = self.orders.lock().await;

        let order = orders.get_mut(&id).expect("Order not found");

        let _ = order.confirmation_sender.send(message).await;
    }

    pub async fn confirm_payment(&self, id: OrderID) {
        self.send_order_message(id, OrderMessage::Completed).await;
    }
}
