use std::sync::{LazyLock, Mutex};
use std::ops::Deref;

static DEBUG_MSGS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::default());
static LOCK_ERR: &str = "could not lock debug message mutex";
static MSG_ERR: &str = "could not find debug message";

pub(crate) fn register(s: String) -> i32 {
	let mut msgs = DEBUG_MSGS.lock().expect(LOCK_ERR);
	let id = msgs.len();
	msgs.push(s);
	id as i32
}

pub(crate) fn recall<F: Fn(&str)>(id: i32, f: F) {
	f(DEBUG_MSGS.lock().expect(LOCK_ERR).get(id as usize).map(Deref::deref).unwrap_or(MSG_ERR))
}
