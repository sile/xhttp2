use futures::{Future, Poll, Async};

use Error;

#[derive(Debug)]
pub struct Finished<T>(Option<T>);
impl<T> Finished<T> {
    pub fn new(item: T) -> Self {
        Finished(Some(item))
    }
    pub fn item(&self) -> &T {
        self.0.as_ref().expect("Already been consumed")
    }
    pub fn item_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("Already been consumed")
    }
}
impl<T> Future for Finished<T> {
    type Item = T;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let item = self.0.take().expect("Cannot poll Finished twice");
        Ok(Async::Ready(item))
    }
}
