//! The login functionality for an instrument.

use crate::error::Result;

/// The log-in state of an instrument.
#[derive(Debug, PartialEq, PartialOrd, Eq)]
pub enum State {
    /// The instrument requires a login action for further communication.
    Needed,
    /// The instrument does _not_ require a login for further communication. Either the
    /// instrument is not password protected OR the instrument is already unlocked for
    /// this client.
    NotNeeded,
}

/// A trait that provides the expected functionality for logging into an instrument.
///
/// This trait is required for a struct to be considered an instrument.
///
/// Some instruments may not have the concept of logging in, for those instruments,
/// simply `impl Login` without any function definitions.
///
/// # Examples
/// ## No Login Functionality Needed
/// ```no_run
/// use tsp_instrument::{instrument::{Login, State}, InstrumentError};
///
/// //This instrument does not have logins
/// struct ExampleInstrument {
///     //...
/// }
///
/// impl Login for ExampleInstrument {}
/// ```
///
/// ## Login Functionality Needed
/// ```no_run
/// use tsp_instrument::{instrument::{Login, State}, InstrumentError};
///
/// struct Example {
///     logged_in: bool,
/// }
///
/// impl Example {
///     fn enter_password(&mut self, token: &[u8]) -> Result<(), InstrumentError>{
///         //...
///         self.logged_in = true;
///         Ok(())
///     }
/// }
///
/// impl Login for Example {
///     fn check_login(&mut self) -> Result<State, InstrumentError> {
///         if self.logged_in {
///             Ok(State::Needed)
///         } else {
///             Ok(State::NotNeeded)
///         }
///     }
///
///     fn login(&mut self, token: &[u8]) -> Result<(), InstrumentError>
///     {
///         self.enter_password(token.as_ref())
///     }
/// }
/// ```
///
pub trait Login {
    /// Check the instrument to see if we need to login it.
    ///
    /// # Returns
    /// - [`State::Needed`]: A login is necessary, therefore [`Login::login`] should be called.
    /// - [`State::NotNeeded`]: A login is not necessary. Proceed with connection.
    ///
    /// # Default `impl`
    /// The default implementation will always return [`State::NotNeeded`] and should
    /// thus _not_ gate a connection to the instrument.
    ///
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn check_login(&mut self) -> Result<State> {
        Ok(State::NotNeeded)
    }

    /// Pass the given token to the instrument for it to authenticate.
    ///
    /// # Default `impl`
    /// The default implementation will always return `Ok(())` and should therefore never
    /// gate a connection to the instrument if called spuriously.
    ///
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn login(&mut self, _token: &[u8]) -> Result<()> {
        Ok(())
    }
}
