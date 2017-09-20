use std;
use handy_async::future::Phase;
use handy_async::io::AsyncIoError;
use trackable::error::TrackableError;
use trackable::error::{ErrorKind as TrackableErrorKind, ErrorKindExt};

/// https://tools.ietf.org/html/rfc7540#section-11.4
#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// Graceful shutdown.
    NoError,

    /// Protocol error detected.
    ProtocolError,

    /// Implementation fault.
    InternalError,

    /// Flow-control limits exceeded.
    FlowControlError,

    /// Settings not acknowledged.
    SettingsTimeout,

    /// Frame received for closed stream.
    StreamClosed,

    /// Frame size incorrect.
    FrameSizeError,

    /// Stream not processed.
    RefusedStream,

    /// Stream cancelled.
    Cancel,

    /// Compression state not updated.
    CompressionError,

    /// TCP connection error for CONNECT method.
    ConnectError,

    /// Processing capacity exceeded.
    EnhanceYourCalm,

    /// Negotiated TLS parameters not acceptable.
    InadequateSecurity,

    /// Use HTTP/1.1 for the request.
    Http11Required,

    // TODO: delete
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
impl<A, B, C, D, E> From<Phase<A, B, C, D, E>> for Error
where
    Error: From<A>,
    Error: From<B>,
    Error: From<C>,
    Error: From<D>,
    Error: From<E>,
{
    fn from(f: Phase<A, B, C, D, E>) -> Self {
        match f {
            Phase::A(e) => track!(Error::from(e), "Phase::A"),
            Phase::B(e) => track!(Error::from(e), "Phase::B"),
            Phase::C(e) => track!(Error::from(e), "Phase::C"),
            Phase::D(e) => track!(Error::from(e), "Phase::D"),
            Phase::E(e) => track!(Error::from(e), "Phase::E"),
        }
    }
}
