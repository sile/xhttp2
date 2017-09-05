use std;
use handy_async::io::AsyncIoError;
use trackable::error::TrackableError;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Invalid,
    Io,
    Other,
}
impl TrackableErrorKind for ErrorKind {}

#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        ErrorKind::Io.cause(f).into()
    }
}
impl<T> From<AsyncIoError<T>> for Error {
    fn from(f: AsyncIoError<T>) -> Self {
        ErrorKind::Io.cause(f.into_error()).into()
    }
}
