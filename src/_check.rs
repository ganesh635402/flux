fn assert_send<T: Send>() {}
fn check() {
    assert_send::<crate::ast::Block>();
}
