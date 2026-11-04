use rstate::{Store, StoreError, StoreResult};

pub fn dispatch<C: 'static, E: 'static, M: 'static>(
    store: &Store<C, E, M>,
    event: E,
    on_error: impl FnOnce(StoreError),
) -> bool {
    handle_result(store.send(event), on_error)
}

pub fn handle_result(result: StoreResult<()>, on_error: impl FnOnce(StoreError)) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => {
            on_error(error);
            false
        }
    }
}
