mod capture_reader;
pub use capture_reader::{Capture, SequentialCapture, ThreadedCapture};

pub use ::mirror_dto::*;

mod prelude {
    mod v1 {
        pub use crate::capture_reader::{Capture, SequentialCapture, ThreadedCapture};
        pub use ::mirror_dto::*;
    }
}
