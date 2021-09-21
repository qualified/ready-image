#![deny(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
mod controller;
mod resource;

pub use controller::run;
pub use resource::ReadyImage;
