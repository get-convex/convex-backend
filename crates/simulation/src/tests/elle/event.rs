use super::{
    ClientId,
    TxId,
    WriteId,
};

#[allow(unused)]
#[derive(Clone, Debug)]
pub enum ElleModelEvent {
    StartRead {
        tx_id: TxId,
        client_id: ClientId,
    },
    FinishRead {
        tx_id: TxId,
        client_id: ClientId,
        write_ids: Option<Vec<WriteId>>,
    },
    StartWrite {
        tx_id: TxId,
        client_id: ClientId,
        write_id: WriteId,
    },
    FinishWrite {
        tx_id: TxId,
        client_id: ClientId,
        write_ids: Vec<WriteId>,
    },
}
