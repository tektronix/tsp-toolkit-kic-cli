use std::ops::{Deref, DerefMut};

use crate::Interface;

pub struct Raw(Box<dyn Interface>);

impl Raw {
    pub fn new(interface: impl Interface + 'static) -> Self {
        Self(Box::new(interface))
    }
}

impl Deref for Raw {
    type Target = Box<dyn Interface>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Raw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
