//! Cooperative cancellation shared by codecs, search and executor owners.
use std::sync::{Arc,atomic::{AtomicBool,Ordering}};
#[derive(Clone)]
enum Flag { Shared(Arc<AtomicBool>), Static(&'static AtomicBool) }
/// A cloneable cancellation request; cancellation is permanent for this token.
#[derive(Clone)]
pub struct Cancellation(Flag);
impl Default for Cancellation { fn default()->Self { Self(Flag::Shared(Arc::new(AtomicBool::new(false)))) } }
impl Cancellation {
    /// Observe a process-owned static signal flag without allocating in a handler.
    pub fn from_static(flag: &'static AtomicBool)->Self { Self(Flag::Static(flag)) }
    fn flag(&self)->&AtomicBool { match &self.0 { Flag::Shared(f)=>f,Flag::Static(f)=>f } }
    /// Request cancellation of all work sharing this token.
    pub fn cancel(&self) { self.flag().store(true,Ordering::Relaxed); }
    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self)->bool { self.flag().load(Ordering::Relaxed) }
    pub(crate) fn token(&self)->zencodec::StopToken { zencodec::StopToken::new(self.clone()) }
}
impl zencodec::enough::Stop for Cancellation {
    fn check(&self)->Result<(),zencodec::enough::StopReason> {
        if self.is_cancelled() { Err(zencodec::enough::StopReason::Cancelled) } else { Ok(()) }
    }
}
